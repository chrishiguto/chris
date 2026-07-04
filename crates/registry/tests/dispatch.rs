//! Macro → registry behavior: manifest emission for a representative
//! signature, prop conversion, and dispatch errors.
//! Run with `cargo test -p registry --features dispatch`.
#![cfg(feature = "dispatch")]

use std::collections::BTreeMap;

use leptos::prelude::*;
use registry::{post_component, DispatchError, Manifest, PropType, PropValue, RegisteredComponent};

#[post_component]
#[component]
fn Kitchen(
    label: String,
    ratio: f64,
    count: i64,
    loud: bool,
    note: Option<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            data-label=label
            data-ratio=ratio.to_string()
            data-count=count.to_string()
            data-loud=loud.to_string()
            data-note=note.unwrap_or_default()
        >
            {children()}
        </div>
    }
}

#[post_component]
#[component]
fn Leaf(tag: String) -> impl IntoView {
    view! { <span>{tag}</span> }
}

fn kitchen() -> &'static RegisteredComponent {
    registry::lookup("Kitchen").expect("Kitchen must be registered")
}

fn props(entries: &[(&str, PropValue)]) -> BTreeMap<String, PropValue> {
    entries
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

fn full_props() -> BTreeMap<String, PropValue> {
    props(&[
        ("label", PropValue::String("hi".into())),
        ("ratio", PropValue::Number(1.5)),
        ("count", PropValue::Number(3.0)),
        ("loud", PropValue::Bool(true)),
    ])
}

#[test]
fn manifest_describes_the_registered_signature() {
    let manifest = registry::manifest();
    let spec = manifest.get("Kitchen").expect("Kitchen in manifest");

    assert!(spec.accepts_children);
    let described: Vec<(&str, PropType, bool)> = spec
        .props
        .iter()
        .map(|p| (p.name.as_str(), p.ty, p.required))
        .collect();
    assert_eq!(
        described,
        vec![
            ("label", PropType::String, true),
            ("ratio", PropType::Float, true),
            ("count", PropType::Int, true),
            ("loud", PropType::Bool, true),
            ("note", PropType::String, false),
        ]
    );

    let leaf = manifest.get("Leaf").expect("Leaf in manifest");
    assert!(!leaf.accepts_children);
}

#[test]
fn manifest_round_trips_through_json() {
    let manifest = registry::manifest();
    let json = serde_json::to_string(&manifest).expect("serialize");
    let back: Manifest = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(manifest, back);
}

#[test]
fn dispatch_converts_props_and_renders_children() {
    let html = (kitchen().render)(&full_props(), "inside".into_any())
        .expect("render")
        .to_html();
    assert!(html.contains("data-label=\"hi\""), "html: {html}");
    assert!(html.contains("data-ratio=\"1.5\""), "html: {html}");
    assert!(html.contains("data-count=\"3\""), "html: {html}");
    assert!(html.contains("data-loud=\"true\""), "html: {html}");
    assert!(html.contains("inside"), "children lost: {html}");
}

#[test]
fn dispatch_passes_optional_prop_when_present() {
    let mut with_note = full_props();
    with_note.insert("note".into(), PropValue::String("psst".into()));
    let html = (kitchen().render)(&with_note, "x".into_any())
        .expect("render")
        .to_html();
    assert!(html.contains("data-note=\"psst\""), "html: {html}");
}

#[test]
fn dispatch_reports_missing_required_prop() {
    let mut missing = full_props();
    missing.remove("label");
    let err = (kitchen().render)(&missing, "x".into_any()).expect_err("must fail");
    assert_eq!(err, DispatchError::MissingProp { prop: "label" });
}

#[test]
fn dispatch_reports_type_mismatch() {
    let mut wrong = full_props();
    wrong.insert("count".into(), PropValue::String("3".into()));
    let err = (kitchen().render)(&wrong, "x".into_any()).expect_err("must fail");
    assert_eq!(
        err,
        DispatchError::TypeMismatch {
            prop: "count",
            expected: PropType::Int,
        }
    );
}

#[test]
fn dispatch_rejects_fractional_value_for_int_prop() {
    let mut fractional = full_props();
    fractional.insert("count".into(), PropValue::Number(3.5));
    let err = (kitchen().render)(&fractional, "x".into_any()).expect_err("must fail");
    assert_eq!(
        err,
        DispatchError::TypeMismatch {
            prop: "count",
            expected: PropType::Int,
        }
    );
}

#[test]
fn lookup_misses_unregistered_names() {
    assert!(registry::lookup("Nope").is_none());
}
