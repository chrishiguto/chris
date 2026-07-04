//! `#[post_component]` (ADR-0005): reads a Leptos component signature and
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
     or Option of one of these, plus `children: Children` (ADR-0005)";

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
        Err(err) => compile_error(&component, err),
    }
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
    ty: Type,
    prop_type: Ident,
    required: bool,
}

fn expand(component: &ItemFn) -> Result<proc_macro2::TokenStream, Error> {
    if !component.sig.generics.params.is_empty() || component.sig.generics.where_clause.is_some() {
        return Err(Error::new(
            component.sig.generics.span(),
            "#[post_component] components cannot be generic (v1 scope, ADR-0005)",
        ));
    }

    let name = &component.sig.ident;
    let name_str = name.to_string();
    let mut props: Vec<Prop> = Vec::new();
    let mut accepts_children = false;

    for input in &component.sig.inputs {
        let FnArg::Typed(arg) = input else {
            return Err(Error::new(input.span(), UNSUPPORTED));
        };
        let Pat::Ident(pat) = arg.pat.as_ref() else {
            return Err(Error::new(arg.pat.span(), UNSUPPORTED));
        };
        let ident = pat.ident.clone();

        if ident == "children" {
            if last_segment_is(&arg.ty, "Children") {
                accepts_children = true;
                continue;
            }
            return Err(Error::new(
                arg.ty.span(),
                "#[post_component] children must be plain `Children` (v1 scope, ADR-0005)",
            ));
        }

        let (inner, required) = match option_inner(&arg.ty) {
            Some(inner) => (inner, false),
            None => (arg.ty.as_ref(), true),
        };
        let Some(prop_type) = scalar_prop_type(inner) else {
            return Err(Error::new(arg.ty.span(), UNSUPPORTED));
        };
        props.push(Prop {
            ident,
            ty: arg.ty.as_ref().clone(),
            prop_type: Ident::new(prop_type, arg.ty.span()),
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
        let prop_type = &prop.prop_type;
        let required = prop.required;
        quote! {
            ::registry::PropInfo {
                name: #prop_name,
                ty: ::registry::PropType::#prop_type,
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

/// Maps a supported scalar Rust type to its `registry::PropType` variant name.
fn scalar_prop_type(ty: &Type) -> Option<&'static str> {
    let Type::Path(path) = ty else { return None };
    let segment = path.path.segments.last()?;
    if !segment.arguments.is_none() {
        return None;
    }
    match segment.ident.to_string().as_str() {
        "String" => Some("String"),
        "f64" => Some("Float"),
        "i64" => Some("Int"),
        "bool" => Some("Bool"),
        _ => None,
    }
}

fn last_segment_is(ty: &Type, ident: &str) -> bool {
    let Type::Path(path) = ty else { return false };
    path.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == ident)
}
