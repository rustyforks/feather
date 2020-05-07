use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Loads a model file.
pub fn from_str(s: &str) -> anyhow::Result<ModelFile> {
    ron::de::from_str(s).map_err(anyhow::Error::from)
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ModelFile {
    Single(Model),
    Multiple(Vec<Model>),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Model {
    Enum {
        name: String,
        variants: Vec<String>,
    },
    Property {
        on: String,
        name: String,
        #[serde(rename = "type")]
        typ: Type,
        mapping: BTreeMap<VecOrOne<String>, ron::Value>,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq)]
#[serde(untagged)]
pub enum VecOrOne<T> {
    Vec(Vec<T>),
    One(T),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Type {
    Slice(Box<Type>),
    #[serde(rename = "u32")]
    U32,
    #[serde(rename = "f64")]
    F64,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "bool")]
    Bool,
    Custom(String),
}
