//! Parses the MDX-syntax authoring subset (ADR-0003) into the versioned
//! IR defined at this crate's root (behind the `parse` feature).
//!
//! Wraps markdown-rs in MDX mode — no custom parser. Everything outside the
//! authoring subset (`import`/`export`, `{expressions}`, non-literal props,
//! reference-style links, …) is reported as a [`Diagnostic`] with a source
//! location instead of silently passing through. The fixture corpus under
//! `fixtures/` doubles as the format specification.

use std::collections::BTreeMap;

use markdown::mdast;
use markdown::message::Place;
use markdown::unist::Position;
use markdown::{Constructs, MdxSignal, ParseOptions};

use crate::{Document, Frontmatter, ListItem, Manifest, Node, PropType, PropValue, SCHEMA_VERSION};

/// A parse or validation error, with a source location when known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Human-readable problem description.
    pub message: String,
    /// File the source came from; set by [`parse_named`].
    pub file: Option<String>,
    /// 1-based line in the source file.
    pub line: Option<usize>,
    /// 1-based column in the source file.
    pub column: Option<usize>,
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let file = self.file.as_deref().unwrap_or("<input>");
        match (self.line, self.column) {
            (Some(line), Some(column)) => write!(f, "{file}:{line}:{column}: {}", self.message),
            (Some(line), None) => write!(f, "{file}:{line}: {}", self.message),
            _ => write!(f, "{file}: {}", self.message),
        }
    }
}

/// Parses one `.mdx` source into a [`Document`].
///
/// Returns every problem found, not just the first; a non-empty diagnostic
/// list means the document must not be published.
pub fn parse(source: &str) -> Result<Document, Vec<Diagnostic>> {
    parse_impl(source, None)
}

/// Like [`parse`], but stamps `file` into every diagnostic.
pub fn parse_named(source: &str, file: &str) -> Result<Document, Vec<Diagnostic>> {
    parse_impl(source, None).map_err(stamp(file))
}

/// [`parse`] plus validation against the component manifest (ADR-0005):
/// unknown components (with did-you-mean), missing/mistyped props, and
/// children handed to childless components all become diagnostics.
///
/// Validation is fused into parsing — not a separate `validate(doc)` pass —
/// because source positions exist only on the markdown tree; the stored AST
/// deliberately carries none (ADR-0002).
pub fn parse_validated(source: &str, manifest: &Manifest) -> Result<Document, Vec<Diagnostic>> {
    parse_impl(source, Some(manifest))
}

/// Like [`parse_validated`], but stamps `file` into every diagnostic.
pub fn parse_validated_named(
    source: &str,
    file: &str,
    manifest: &Manifest,
) -> Result<Document, Vec<Diagnostic>> {
    parse_impl(source, Some(manifest)).map_err(stamp(file))
}

fn stamp(file: &str) -> impl Fn(Vec<Diagnostic>) -> Vec<Diagnostic> + '_ {
    move |diags| {
        diags
            .into_iter()
            .map(|d| Diagnostic {
                file: Some(file.to_string()),
                ..d
            })
            .collect()
    }
}

fn parse_impl(source: &str, manifest: Option<&Manifest>) -> Result<Document, Vec<Diagnostic>> {
    let mdast = markdown::to_mdast(source, &parse_options()).map_err(|msg| {
        let (line, column) = match msg.place.as_deref() {
            Some(Place::Position(pos)) => (Some(pos.start.line), Some(pos.start.column)),
            Some(Place::Point(point)) => (Some(point.line), Some(point.column)),
            None => (None, None),
        };
        vec![Diagnostic {
            message: msg.reason.clone(),
            file: None,
            line,
            column,
        }]
    })?;

    let mdast::Node::Root(root) = mdast else {
        unreachable!("to_mdast always returns a Root");
    };

    let mut converter = Converter {
        diags: Vec::new(),
        manifest,
    };
    let (frontmatter, body) = converter.split_frontmatter(root.children);
    let ast = converter.blocks(body);

    match (frontmatter, converter.diags.is_empty()) {
        (Some(frontmatter), true) => Ok(Document {
            schema_version: SCHEMA_VERSION,
            frontmatter,
            ast,
        }),
        _ => Err(converter.diags),
    }
}

/// MDX constructs + YAML frontmatter. The permissive esm/expression hooks
/// exist only so markdown-rs *recognizes* those constructs — the converter
/// then rejects the resulting nodes with proper diagnostics (ADR-0003).
fn parse_options() -> ParseOptions {
    ParseOptions {
        constructs: Constructs {
            frontmatter: true,
            ..Constructs::mdx()
        },
        mdx_esm_parse: Some(Box::new(|_| MdxSignal::Ok)),
        mdx_expression_parse: Some(Box::new(|_, _| MdxSignal::Ok)),
        ..ParseOptions::default()
    }
}

struct Converter<'m> {
    diags: Vec<Diagnostic>,
    /// When present, component nodes are validated against it (ADR-0005).
    manifest: Option<&'m Manifest>,
}

impl Converter<'_> {
    fn error(&mut self, message: impl Into<String>, position: Option<&Position>) {
        self.diags.push(Diagnostic {
            message: message.into(),
            file: None,
            line: position.map(|p| p.start.line),
            column: position.map(|p| p.start.column),
        });
    }

    fn split_frontmatter(
        &mut self,
        children: Vec<mdast::Node>,
    ) -> (Option<Frontmatter>, Vec<mdast::Node>) {
        let mut iter = children.into_iter();
        match iter.next() {
            Some(mdast::Node::Yaml(yaml)) => (self.frontmatter(&yaml), iter.collect()),
            Some(mdast::Node::Toml(toml)) => {
                self.error(
                    "frontmatter must be YAML (`---` fences), not TOML",
                    toml.position.as_ref(),
                );
                (None, iter.collect())
            }
            // Missing frontmatter: keep the first node in the body.
            first => {
                self.diags.push(Diagnostic {
                    message: "missing frontmatter: posts must start with `---` YAML frontmatter \
                         declaring at least `title` and `date`"
                        .into(),
                    file: None,
                    line: Some(1),
                    column: Some(1),
                });
                (None, first.into_iter().chain(iter).collect())
            }
        }
    }

    fn frontmatter(&mut self, yaml: &mdast::Yaml) -> Option<Frontmatter> {
        match serde_yaml::from_str::<Frontmatter>(&yaml.value) {
            Ok(frontmatter) => Some(frontmatter),
            Err(err) => {
                // serde_yaml locations are relative to the YAML block, which
                // starts one line below the opening `---` fence.
                let fence_line = yaml.position.as_ref().map_or(1, |p| p.start.line);
                let (line, column) = err.location().map_or((Some(fence_line), None), |loc| {
                    (Some(fence_line + loc.line()), Some(loc.column()))
                });
                self.diags.push(Diagnostic {
                    message: format!("malformed frontmatter: {err}"),
                    file: None,
                    line,
                    column,
                });
                None
            }
        }
    }

    fn blocks(&mut self, nodes: Vec<mdast::Node>) -> Vec<Node> {
        nodes
            .into_iter()
            .filter_map(|node| self.node(node))
            .collect()
    }

    fn node(&mut self, node: mdast::Node) -> Option<Node> {
        match node {
            mdast::Node::Heading(h) => Some(Node::Heading {
                level: h.depth,
                children: self.blocks(h.children),
            }),
            mdast::Node::Paragraph(p) => Some(Node::Paragraph {
                children: self.blocks(p.children),
            }),
            mdast::Node::Text(t) => Some(Node::Text { value: t.value }),
            mdast::Node::Emphasis(e) => Some(Node::Emphasis {
                children: self.blocks(e.children),
            }),
            mdast::Node::Strong(s) => Some(Node::Strong {
                children: self.blocks(s.children),
            }),
            mdast::Node::InlineCode(c) => Some(Node::InlineCode { value: c.value }),
            mdast::Node::Link(l) => Some(Node::Link {
                url: l.url,
                title: l.title,
                children: self.blocks(l.children),
            }),
            mdast::Node::Image(i) => Some(Node::Image {
                url: i.url,
                alt: i.alt,
                title: i.title,
            }),
            mdast::Node::List(l) => {
                let items = l
                    .children
                    .into_iter()
                    .filter_map(|child| match child {
                        mdast::Node::ListItem(item) => Some(ListItem {
                            children: self.blocks(item.children),
                        }),
                        other => {
                            self.unsupported(&other);
                            None
                        }
                    })
                    .collect();
                Some(Node::List {
                    ordered: l.ordered,
                    start: l.start,
                    items,
                })
            }
            mdast::Node::Code(c) => Some(Node::CodeBlock {
                lang: c.lang,
                text: c.value,
            }),
            mdast::Node::Blockquote(b) => Some(Node::Blockquote {
                children: self.blocks(b.children),
            }),
            mdast::Node::ThematicBreak(_) => Some(Node::ThematicBreak),
            mdast::Node::Break(_) => Some(Node::Break),
            mdast::Node::MdxJsxFlowElement(el) => {
                self.jsx(el.name, el.attributes, el.children, el.position)
            }
            mdast::Node::MdxJsxTextElement(el) => {
                self.jsx(el.name, el.attributes, el.children, el.position)
            }
            mdast::Node::MdxjsEsm(esm) => {
                let keyword = if esm.value.trim_start().starts_with("export") {
                    "export"
                } else {
                    "import"
                };
                self.error(
                    format!(
                        "`{keyword}` statements are not supported: posts are an MDX-syntax \
                         subset without JavaScript; component names resolve through the \
                         registry instead"
                    ),
                    esm.position.as_ref(),
                );
                None
            }
            mdast::Node::MdxFlowExpression(expr) => {
                self.expression_diag(&expr.value, expr.position.as_ref());
                None
            }
            mdast::Node::MdxTextExpression(expr) => {
                self.expression_diag(&expr.value, expr.position.as_ref());
                None
            }
            other => {
                self.unsupported(&other);
                None
            }
        }
    }

    fn expression_diag(&mut self, value: &str, position: Option<&Position>) {
        self.error(
            format!(
                "`{{{value}}}` is not supported: JS expressions are outside the authoring \
                 subset; props take scalar literals and rich content goes in children"
            ),
            position,
        );
    }

    fn unsupported(&mut self, node: &mdast::Node) {
        let what = match node {
            mdast::Node::Definition(_)
            | mdast::Node::LinkReference(_)
            | mdast::Node::ImageReference(_) => {
                "reference-style links are outside the authoring subset; use inline \
                 `[text](url)` links"
            }
            _ => "this syntax is outside the authoring subset",
        };
        self.error(what, node.position());
    }

    fn jsx(
        &mut self,
        name: Option<String>,
        attributes: Vec<mdast::AttributeContent>,
        children: Vec<mdast::Node>,
        position: Option<Position>,
    ) -> Option<Node> {
        let Some(name) = name else {
            self.error(
                "JSX fragments (`<>…</>`) are not supported; use markdown or a named tag",
                position.as_ref(),
            );
            return None;
        };
        if name.contains(['.', ':']) {
            self.error(
                format!(
                    "`<{name}>` is not supported: component names are plain PascalCase \
                     identifiers resolved through the registry"
                ),
                position.as_ref(),
            );
            return None;
        }

        if name.starts_with(|c: char| c.is_ascii_uppercase()) {
            let props = self.component_props(&name, attributes, position.as_ref());
            let children = self.blocks(children);
            self.validate_component(&name, &props, !children.is_empty(), position.as_ref());
            Some(Node::Component {
                name,
                props,
                children,
            })
        } else {
            let attrs = self.html_attrs(&name, attributes, position.as_ref());
            Some(Node::Html {
                tag: name,
                attrs,
                children: self.blocks(children),
            })
        }
    }

    /// No-op unless a manifest was supplied (plain [`parse`] stays untyped).
    fn validate_component(
        &mut self,
        name: &str,
        props: &BTreeMap<String, PropValue>,
        has_children: bool,
        position: Option<&Position>,
    ) {
        let Some(manifest) = self.manifest else {
            return;
        };
        let Some(spec) = manifest.get(name) else {
            let hint = suggest(name, manifest.names())
                .map(|candidate| format!("; did you mean `<{candidate}>`?"))
                .unwrap_or_else(|| {
                    ": it is not in the component registry (see CONTENT.md for the vocabulary)"
                        .into()
                });
            self.error(format!("unknown component `<{name}>`{hint}"), position);
            return;
        };

        for prop in spec.props.iter().filter(|p| p.required) {
            if !props.contains_key(&prop.name) {
                self.error(
                    format!(
                        "`<{name}>` is missing required prop `{prop_name}` ({ty})",
                        prop_name = prop.name,
                        ty = prop.ty.describe(),
                    ),
                    position,
                );
            }
        }

        for (prop_name, value) in props {
            match spec.prop(prop_name) {
                None => {
                    let hint = suggest(prop_name, spec.props.iter().map(|p| p.name.as_str()))
                        .map(|candidate| format!("; did you mean `{candidate}`?"))
                        .unwrap_or_default();
                    self.error(
                        format!("unknown prop `{prop_name}` on `<{name}>`{hint}"),
                        position,
                    );
                }
                Some(spec_prop) if !spec_prop.ty.matches(value) => {
                    self.error(
                        format!(
                            "prop `{prop_name}` on `<{name}>` expects {expected}, got {got}{hint}",
                            expected = spec_prop.ty.describe(),
                            got = show_prop_value(value),
                            hint = mismatch_hint(prop_name, spec_prop.ty, value),
                        ),
                        position,
                    );
                }
                Some(_) => {}
            }
        }

        if has_children && !spec.accepts_children {
            self.error(format!("`<{name}>` does not accept children"), position);
        }
    }

    fn component_props(
        &mut self,
        component: &str,
        attributes: Vec<mdast::AttributeContent>,
        position: Option<&Position>,
    ) -> BTreeMap<String, PropValue> {
        attributes
            .into_iter()
            .filter_map(|attr| match attr {
                mdast::AttributeContent::Expression(_) => {
                    self.error(
                        format!(
                            "`{{...}}` spread attributes on `<{component}>` are not \
                             supported: props must be named scalar literals"
                        ),
                        position,
                    );
                    None
                }
                mdast::AttributeContent::Property(prop) => {
                    let value = match prop.value {
                        None => PropValue::Bool(true),
                        Some(mdast::AttributeValue::Literal(s)) => PropValue::String(s),
                        Some(mdast::AttributeValue::Expression(expr)) => {
                            self.scalar_literal(component, &prop.name, &expr.value, position)?
                        }
                    };
                    Some((prop.name, value))
                }
            })
            .collect()
    }

    /// Braced prop values may only be number or boolean literals: strings use
    /// plain quotes, and anything else is code, which the subset forbids.
    fn scalar_literal(
        &mut self,
        component: &str,
        prop: &str,
        raw: &str,
        position: Option<&Position>,
    ) -> Option<PropValue> {
        let value = raw.trim();
        match value {
            "true" => return Some(PropValue::Bool(true)),
            "false" => return Some(PropValue::Bool(false)),
            _ => {}
        }
        if let Ok(number) = value.parse::<f64>() {
            if number.is_finite() && !value.contains(|c: char| c.is_ascii_alphabetic()) {
                return Some(PropValue::Number(number));
            }
        }
        let hint = if value.starts_with(['"', '\'']) {
            "; for strings, drop the braces: prop=\"value\""
        } else {
            ""
        };
        self.error(
            format!(
                "non-literal prop `{prop}={{{raw}}}` on `<{component}>`: props must be \
                 scalar literals — a quoted string, a number, or true/false{hint}"
            ),
            position,
        );
        None
    }

    fn html_attrs(
        &mut self,
        tag: &str,
        attributes: Vec<mdast::AttributeContent>,
        position: Option<&Position>,
    ) -> BTreeMap<String, String> {
        attributes
            .into_iter()
            .filter_map(|attr| match attr {
                mdast::AttributeContent::Expression(_) => {
                    self.error(
                        format!(
                            "`{{...}}` spread attributes on `<{tag}>` are not supported: \
                             HTML attributes must be string literals"
                        ),
                        position,
                    );
                    None
                }
                mdast::AttributeContent::Property(prop) => match prop.value {
                    None => Some((prop.name, String::new())),
                    Some(mdast::AttributeValue::Literal(s)) => Some((prop.name, s)),
                    Some(mdast::AttributeValue::Expression(expr)) => {
                        self.error(
                            format!(
                                "non-literal attribute `{name}={{{value}}}` on `<{tag}>`: \
                                 HTML attributes must be string literals",
                                name = prop.name,
                                value = expr.value,
                            ),
                            position,
                        );
                        None
                    }
                },
            })
            .collect()
    }
}

/// Prop value as it would appear in source, for diagnostics.
fn show_prop_value(value: &PropValue) -> String {
    match value {
        PropValue::String(s) => format!("`\"{s}\"`"),
        PropValue::Number(n) => format!("`{n}`"),
        PropValue::Bool(b) => format!("`{b}`"),
    }
}

/// Syntax nudge for the two classic quoting mistakes: quoting a number/bool
/// (needs braces) or bracing what should be a plain quoted string.
fn mismatch_hint(prop: &str, expected: PropType, value: &PropValue) -> String {
    match (expected, value) {
        (PropType::String, PropValue::Number(n)) => {
            format!("; strings use quotes: {prop}=\"{n}\"")
        }
        (PropType::String, PropValue::Bool(b)) => {
            format!("; strings use quotes: {prop}=\"{b}\"")
        }
        (expected, PropValue::String(s)) if reads_as(expected, s) => {
            format!("; numbers and booleans use braces: {prop}={{{s}}}")
        }
        _ => String::new(),
    }
}

/// Whether a quoted string would be a valid braced literal of `ty`.
fn reads_as(ty: PropType, s: &str) -> bool {
    match ty {
        PropType::Int => s.parse::<i64>().is_ok(),
        PropType::Float => s.parse::<f64>().is_ok(),
        PropType::Bool => matches!(s, "true" | "false"),
        PropType::String => false,
    }
}

/// Closest candidate within edit distance 2 (case-insensitive), for
/// "did you mean" suggestions.
fn suggest<'a>(target: &str, candidates: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    candidates
        .map(|candidate| {
            (
                levenshtein(&target.to_lowercase(), &candidate.to_lowercase()),
                candidate,
            )
        })
        .filter(|(distance, _)| *distance <= 2)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, candidate)| candidate)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let b_chars: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b_chars.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, cb) in b_chars.iter().enumerate() {
            let up = row[j + 1];
            row[j + 1] = if ca == *cb {
                diagonal
            } else {
                1 + diagonal.min(up).min(row[j])
            };
            diagonal = up;
        }
    }
    row[b_chars.len()]
}
