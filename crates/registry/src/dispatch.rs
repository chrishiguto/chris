//! Runtime dispatch: `#[post_component]` registrations collected via
//! `inventory`, looked up by name at render time.

use std::collections::BTreeMap;

use content::PropValue;
use leptos::prelude::AnyView;

use crate::{integral, ComponentSpec, Manifest, PropSpec, PropType};

pub use inventory;
pub use registry_macro::post_component;

/// One `#[post_component]` registration; constructed by the macro, not by hand.
pub struct RegisteredComponent {
    pub name: &'static str,
    pub props: &'static [PropInfo],
    pub accepts_children: bool,
    /// Converts string-keyed props and pre-rendered children into a typed
    /// component call.
    pub render: fn(&BTreeMap<String, PropValue>, AnyView) -> Result<AnyView, DispatchError>,
}

/// Const-constructible prop metadata, so registrations can be `static`.
pub struct PropInfo {
    pub name: &'static str,
    pub ty: PropType,
    pub required: bool,
}

inventory::collect!(RegisteredComponent);

pub fn lookup(name: &str) -> Option<&'static RegisteredComponent> {
    inventory::iter::<RegisteredComponent>().find(|c| c.name == name)
}

/// Builds the manifest from every registration linked into this binary.
pub fn manifest() -> Manifest {
    let mut components: Vec<ComponentSpec> = inventory::iter::<RegisteredComponent>()
        .map(|c| ComponentSpec {
            name: c.name.to_string(),
            props: c
                .props
                .iter()
                .map(|p| PropSpec {
                    name: p.name.to_string(),
                    ty: p.ty,
                    required: p.required,
                })
                .collect(),
            accepts_children: c.accepts_children,
        })
        .collect();
    // Inventory iteration order is link-dependent; sort for determinism.
    components.sort_by(|a, b| a.name.cmp(&b.name));
    Manifest { components }
}

/// Unreachable for content validated at publish time; exists so bad KV data
/// fails loudly, never silently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchError {
    MissingProp {
        prop: &'static str,
    },
    TypeMismatch {
        prop: &'static str,
        expected: PropType,
    },
}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DispatchError::MissingProp { prop } => write!(f, "missing required prop `{prop}`"),
            DispatchError::TypeMismatch { prop, expected } => {
                write!(f, "prop `{prop}` expects {}", expected.describe())
            }
        }
    }
}

impl std::error::Error for DispatchError {}

/// A prop type the macro can convert a [`PropValue`] into.
pub trait FromPropValue: Sized {
    const TYPE: PropType;
    fn from_prop(value: &PropValue) -> Option<Self>;
}

impl FromPropValue for String {
    const TYPE: PropType = PropType::String;
    fn from_prop(value: &PropValue) -> Option<Self> {
        match value {
            PropValue::String(s) => Some(s.clone()),
            _ => None,
        }
    }
}

impl FromPropValue for f64 {
    const TYPE: PropType = PropType::Float;
    fn from_prop(value: &PropValue) -> Option<Self> {
        match value {
            PropValue::Number(n) => Some(*n),
            _ => None,
        }
    }
}

impl FromPropValue for i64 {
    const TYPE: PropType = PropType::Int;
    fn from_prop(value: &PropValue) -> Option<Self> {
        match value {
            PropValue::Number(n) => integral(*n),
            _ => None,
        }
    }
}

impl FromPropValue for bool {
    const TYPE: PropType = PropType::Bool;
    fn from_prop(value: &PropValue) -> Option<Self> {
        match value {
            PropValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

/// Extracts a required prop for macro-generated glue.
pub fn required_prop<T: FromPropValue>(
    props: &BTreeMap<String, PropValue>,
    name: &'static str,
) -> Result<T, DispatchError> {
    match props.get(name) {
        None => Err(DispatchError::MissingProp { prop: name }),
        Some(value) => T::from_prop(value).ok_or(DispatchError::TypeMismatch {
            prop: name,
            expected: T::TYPE,
        }),
    }
}

/// Extracts an `Option<T>` prop for macro-generated glue.
pub fn optional_prop<T: FromPropValue>(
    props: &BTreeMap<String, PropValue>,
    name: &'static str,
) -> Result<Option<T>, DispatchError> {
    props
        .get(name)
        .map(|value| {
            T::from_prop(value).ok_or(DispatchError::TypeMismatch {
                prop: name,
                expected: T::TYPE,
            })
        })
        .transpose()
}
