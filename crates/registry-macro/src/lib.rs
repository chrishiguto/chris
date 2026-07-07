//! `#[post_component]`: reads a Leptos component signature and
//! generates the registry glue — string-attr → typed-prop conversion plus an
//! `inventory` registration carrying the component's manifest entry.
//!
//! v1 scope is deliberately bounded: scalar props (`String`, `f64`, `i64`,
//! `bool`), `Option<…>` of those for MDX-optional props, and markdown
//! `children`. The macro must sit *above* `#[component]`/`#[island]` so it
//! sees the original signature, and the consuming crate must depend on
//! `registry` (with the `dispatch` feature) and `leptos` under those names.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{Error, FnArg, Ident, ItemFn, Pat, Type};

const UNSUPPORTED: &str = "#[post_component] v1 supports props of type String, f64, i64, bool, \
     or Option of one of these, plus `children: Children`";

#[proc_macro_attribute]
pub fn post_component(attr: TokenStream, item: TokenStream) -> TokenStream {
    let component = syn::parse_macro_input!(item as ItemFn);
    if !attr.is_empty() {
        return compile_error(
            &component,
            Error::new(Span::call_site(), "#[post_component] takes no arguments"),
        );
    }
    match expand(&component) {
        Ok(glue) => {
            let mut out = quote! { #component };
            out.extend(glue);
            out.into()
        }
        Err(err) => compile_error(&component, ordering_hint(&component, err)),
    }
}

/// The most likely authoring mistake is putting `#[post_component]` *below*
/// `#[component]` — the macro then sees leptos's expanded output and rejects
/// it with a baffling prop-type error. When no component/island attribute
/// remains below us, say so.
fn ordering_hint(component: &ItemFn, err: Error) -> Error {
    let has_component_attr = component.attrs.iter().any(|attr| {
        attr.path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "component" || segment.ident == "island")
    });
    if has_component_attr {
        return err;
    }
    Error::new(
        err.span(),
        format!(
            "{err}\n= help: #[post_component] must sit above #[component] or #[island] \
             (no such attribute was found below it)"
        ),
    )
}

/// Emits the error alongside the untouched fn so the only diagnostic the
/// author sees is ours (no cascading unknown-identifier noise).
fn compile_error(component: &ItemFn, err: Error) -> TokenStream {
    let err = err.to_compile_error();
    quote! {
        #err
        #component
    }
    .into()
}

struct Prop {
    ident: Ident,
    /// The declared type, e.g. `Option<i64>`.
    ty: Type,
    /// The scalar inside an `Option`, or the declared type itself; its
    /// `FromPropValue::TYPE` is the manifest entry, so macro and dispatch
    /// can never disagree about a prop's type.
    inner: Type,
    required: bool,
}

fn expand(component: &ItemFn) -> Result<proc_macro2::TokenStream, Error> {
    if !component.sig.generics.params.is_empty() || component.sig.generics.where_clause.is_some() {
        return Err(Error::new(
            component.sig.generics.span(),
            "#[post_component] components cannot be generic (v1 scope)",
        ));
    }

    let name = &component.sig.ident;
    let name_str = name.to_string();
    let mut props: Vec<Prop> = Vec::new();
    let mut accepts_children = false;

    for input in &component.sig.inputs {
        let FnArg::Typed(arg) = input else {
            return Err(Error::new(
                input.span(),
                "#[post_component] components cannot take `self`",
            ));
        };
        let Pat::Ident(pat) = arg.pat.as_ref() else {
            return Err(Error::new(
                arg.pat.span(),
                "#[post_component] props must be plain identifiers, not patterns",
            ));
        };
        let ident = pat.ident.clone();

        if ident == "children" {
            if last_segment_is(&arg.ty, "Children") {
                accepts_children = true;
                continue;
            }
            return Err(Error::new(
                arg.ty.span(),
                "#[post_component] children must be plain `Children` (v1 scope)",
            ));
        }

        let (inner, required) = match option_inner(&arg.ty) {
            Some(inner) => (inner, false),
            None => (arg.ty.as_ref(), true),
        };
        if !is_supported_scalar(inner) {
            return Err(Error::new(arg.ty.span(), UNSUPPORTED));
        }
        props.push(Prop {
            ident,
            ty: arg.ty.as_ref().clone(),
            inner: inner.clone(),
            required,
        });
    }

    let conversions = props.iter().map(|prop| {
        let Prop { ident, ty, .. } = prop;
        let prop_name = ident.to_string();
        let extract = if prop.required {
            quote! { ::registry::required_prop(__props, #prop_name)? }
        } else {
            quote! { ::registry::optional_prop(__props, #prop_name)? }
        };
        quote! { let #ident: #ty = #extract; }
    });

    let prop_infos = props.iter().map(|prop| {
        let prop_name = prop.ident.to_string();
        let inner = &prop.inner;
        let required = prop.required;
        quote! {
            ::registry::PropInfo {
                name: #prop_name,
                ty: <#inner as ::registry::FromPropValue>::TYPE,
                required: #required,
            }
        }
    });

    let prop_args = props.iter().map(|prop| {
        let ident = &prop.ident;
        quote! { #ident=#ident }
    });

    let children_param = format_ident!("{}children", if accepts_children { "__" } else { "_" });
    let invocation = if accepts_children {
        quote! { ::leptos::view! { <#name #(#prop_args)*>{#children_param}</#name> } }
    } else {
        quote! { ::leptos::view! { <#name #(#prop_args)* /> } }
    };

    Ok(quote! {
        const _: () = {
            fn __render(
                __props: &::std::collections::BTreeMap<::std::string::String, ::registry::PropValue>,
                #children_param: ::leptos::prelude::AnyView,
            ) -> ::core::result::Result<::leptos::prelude::AnyView, ::registry::DispatchError> {
                #(#conversions)*
                ::core::result::Result::Ok(::leptos::prelude::IntoAny::into_any(#invocation))
            }
            ::registry::inventory::submit! {
                ::registry::RegisteredComponent {
                    name: #name_str,
                    props: &[#(#prop_infos),*],
                    accepts_children: #accepts_children,
                    render: __render,
                }
            }
        };
    })
}

/// `Option<T>` → `Some(T)`; anything else → `None`.
fn option_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else { return None };
    let segment = path.path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    match args.args.first()? {
        syn::GenericArgument::Type(inner) if args.args.len() == 1 => Some(inner),
        _ => None,
    }
}

/// The curated-error allowlist; the manifest type itself comes from
/// `FromPropValue::TYPE` in the generated code.
fn is_supported_scalar(ty: &Type) -> bool {
    let Type::Path(path) = ty else { return false };
    let Some(segment) = path.path.segments.last() else {
        return false;
    };
    segment.arguments.is_none()
        && matches!(
            segment.ident.to_string().as_str(),
            "String" | "f64" | "i64" | "bool"
        )
}

fn last_segment_is(ty: &Type, ident: &str) -> bool {
    let Type::Path(path) = ty else { return false };
    path.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == ident)
}
