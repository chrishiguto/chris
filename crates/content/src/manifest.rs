//! The component vocabulary types (ADR-0005): serde data describing which
//! components exist, with which props. Defined here — the shared vocabulary
//! crate — so the parser can validate against them without depending on the
//! registry; the `registry` crate *produces* a [`Manifest`] from its
//! `inventory` registrations and re-exports these types.

use serde::{Deserialize, Serialize};

use crate::PropValue;

/// The scalar prop types the v1 macro supports (ADR-0005's bounded scope).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropType {
    /// `String` — a quoted attribute: `kind="warning"`.
    String,
    /// `f64` — a braced number: `ratio={1.6}`.
    Float,
    /// `i64` — a braced integer: `initial={3}`.
    Int,
    /// `bool` — braced `{true}`/`{false}`, or bare (`autoplay` ⇒ true).
    Bool,
}

impl PropType {
    /// Whether a parsed prop value satisfies this declared type.
    pub fn matches(self, value: &PropValue) -> bool {
        match (self, value) {
            (PropType::String, PropValue::String(_)) => true,
            (PropType::Float, PropValue::Number(_)) => true,
            (PropType::Int, PropValue::Number(n)) => integral(*n).is_some(),
            (PropType::Bool, PropValue::Bool(_)) => true,
            _ => false,
        }
    }

    /// Human phrase for diagnostics: "expects {describe()}".
    pub fn describe(self) -> &'static str {
        match self {
            PropType::String => "a string",
            PropType::Float => "a number",
            PropType::Int => "an integer",
            PropType::Bool => "a boolean",
        }
    }
}

/// Converts an exactly-integral `f64` to `i64` (prop numbers arrive as `f64`
/// from JSON/MDX). Formats through decimal text instead of an `as` cast so
/// out-of-range values fail instead of silently truncating.
pub fn integral(n: f64) -> Option<i64> {
    (n.fract() == 0.0)
        .then(|| format!("{n:.0}").parse().ok())
        .flatten()
}

/// The full registered vocabulary; what validation checks posts against.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    /// Sorted by component name for deterministic output.
    pub components: Vec<ComponentSpec>,
}

impl Manifest {
    pub fn get(&self, name: &str) -> Option<&ComponentSpec> {
        self.components.iter().find(|c| c.name == name)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.components.iter().map(|c| c.name.as_str())
    }
}

/// One registered component: its name, props, and whether it takes children.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentSpec {
    pub name: String,
    pub props: Vec<PropSpec>,
    pub accepts_children: bool,
}

impl ComponentSpec {
    pub fn prop(&self, name: &str) -> Option<&PropSpec> {
        self.props.iter().find(|p| p.name == name)
    }
}

/// One prop of a registered component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropSpec {
    pub name: String,
    pub ty: PropType,
    /// `false` when the component declares the prop as `Option<T>`.
    pub required: bool,
}
