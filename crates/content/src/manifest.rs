//! Serde types describing the registered component vocabulary, defined here
//! so the parser can validate against them without depending on the registry.

use serde::{Deserialize, Serialize};

use crate::PropValue;

/// The scalar prop types the macro supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropType {
    String,
    /// `f64`.
    Float,
    /// `i64`.
    Int,
    /// Braced `{true}`/`{false}`, or bare (`autoplay` ⇒ true).
    Bool,
}

impl PropType {
    pub fn matches(self, value: &PropValue) -> bool {
        match (self, value) {
            (PropType::String, PropValue::String(_)) => true,
            (PropType::Float, PropValue::Number(_)) => true,
            (PropType::Int, PropValue::Number(n)) => integral(*n).is_some(),
            (PropType::Bool, PropValue::Bool(_)) => true,
            _ => false,
        }
    }

    /// Human phrase for diagnostics.
    pub fn describe(self) -> &'static str {
        match self {
            PropType::String => "a string",
            PropType::Float => "a number",
            PropType::Int => "an integer",
            PropType::Bool => "a boolean",
        }
    }
}

/// Exactly-integral `f64` → `i64`. Goes through decimal text, not an `as`
/// cast, so out-of-range values fail instead of silently truncating.
pub fn integral(n: f64) -> Option<i64> {
    (n.fract() == 0.0)
        .then(|| format!("{n:.0}").parse().ok())
        .flatten()
}

/// The full registered vocabulary; what validation checks posts against.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    /// Sorted by name for deterministic output.
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropSpec {
    pub name: String,
    pub ty: PropType,
    /// `false` when the component declares the prop as `Option<T>`.
    pub required: bool,
}
