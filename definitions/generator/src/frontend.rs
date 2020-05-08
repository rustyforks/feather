use crate::model::{Model, ModelFile, Type, VecOrOne};
use anyhow::Context;
use heck::CamelCase;
use itertools::Either;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::BTreeMap;
use std::ops::Range;

pub struct DataFile {
    pub contents: String,
    pub name: String,
}

/// Creates a `Data` from a slice
/// of data files.
pub fn from_slice(files: &[DataFile]) -> anyhow::Result<Data> {
    let mut data = Data::default();

    for file in files {
        add_to_data(&file.contents, &file.name, &mut data)
            .with_context(|| format!("failed to load data file `{}`", file.name))?;
    }
    expand(&mut data).context("failed to expand expressions")?;

    Ok(data)
}

fn add_to_data(file: &str, file_name: &str, data: &mut Data) -> anyhow::Result<()> {
    let model = crate::model::from_str(file)?;

    let iter = match model {
        ModelFile::Single(m) => Either::Left(std::iter::once(m)),
        ModelFile::Multiple(vec) => Either::Right(vec.into_iter()),
    };

    for model in iter {
        match &model {
            Model::Enum { name, variants } => {
                let existing = data.enums.entry(name.clone()).or_default();

                existing.name = name.to_owned();
                existing.name_camel_case = name.to_camel_case();
                existing.variants_camel_case = variants.iter().map(|v| v.to_camel_case()).collect();
                existing.variants = variants.clone();
                existing.file = file_name.to_owned();
            }
            Model::Property {
                on,
                name,
                typ,
                mapping,
            } => {
                let existing = data.enums.entry(on.clone()).or_default();

                let pf = Property {
                    name: name.clone(),
                    typ: typ.clone(),
                    mapping: mapping
                        .iter()
                        .flat_map(|(_keys, value)| {
                            let keys;
                            match _keys {
                                VecOrOne::Vec(vec) => keys = vec.clone(),
                                VecOrOne::One(x) => keys = vec![x.clone()],
                            };

                            keys.iter()
                                .map(|key| {
                                    (
                                        key.clone(),
                                        Value::from_ron(value.clone(), typ.clone()).unwrap(),
                                    )
                                })
                                .collect::<Vec<_>>()
                        })
                        .collect(),
                };

                if existing.properties.insert(name.clone(), pf).is_some() {
                    anyhow::bail!("property `{}` defined twice", name);
                }
            }
        }
    }

    Ok(())
}

#[derive(Default, Debug)]
pub struct Data {
    /// Mapping from enum names => enums
    pub enums: BTreeMap<String, Enum>,
}

#[derive(Debug, Default)]
pub struct Enum {
    pub name: String,
    pub name_camel_case: String,

    pub variants: Vec<String>,
    pub variants_camel_case: Vec<String>,

    /// Mapping from property names => properties
    pub properties: BTreeMap<String, Property>,

    /// File name where this enum is described
    pub file: String,
}

#[derive(Debug)]
pub struct Property {
    pub name: String,
    pub typ: Type,
    /// Mapping from variant names => values
    pub mapping: BTreeMap<String, Value>,
}

#[derive(Clone, Debug)]
pub enum Value {
    U32(u32),
    F64(f64),
    String(String),
    Slice(Vec<Value>),
    Bool(bool),
    /// custom type - name of enum variant
    Custom(String),
}

impl Value {
    pub fn from_ron(r: ron::Value, typ: Type) -> anyhow::Result<Self> {
        use ron::Value as Ron;

        Ok(match r {
            Ron::Number(n) => match typ {
                Type::U32 => Value::U32(n.get().round() as u32),
                Type::F64 => Value::F64(n.get()),
                t => anyhow::bail!("value {:?} is not a valid instance of type {:?}", t, r),
            },
            Ron::String(s) if typ == Type::String => Value::String(s),
            Ron::String(s) => Value::Custom(s),
            Ron::Seq(values) => Value::Slice(
                values
                    .into_iter()
                    .map(|v| Value::from_ron(v, typ.clone()))
                    .collect::<anyhow::Result<Vec<_>>>()?,
            ),
            Ron::Bool(x) => Value::Bool(x),
            r => anyhow::bail!("value {:?} is not supported for type {:?}", r, typ),
        })
    }
}

/// Expands expresisons in properties and enums.
fn expand(data: &mut Data) -> anyhow::Result<()> {
    // enum variants
    let mut replacements = vec![];
    for e in data.enums.values() {
        let new_variants = e
            .variants
            .iter()
            .flat_map(|variant| expand_expr(variant, data).unwrap())
            .collect::<Vec<_>>();
        replacements.push((e.name.to_owned(), new_variants));
    }

    for (e, new_variants) in replacements {
        data.enums.get_mut(&e).unwrap().variants = new_variants;
    }

    // property keys + values
    let mut replacements = vec![];
    for e in data.enums.values() {
        for prop in e.properties.values() {
            match prop.typ {
                Type::Custom(_) => (),
                _ => continue,
            }

            let mut new_keys = vec![];
            let mut new_values = vec![];
            for (key, value) in &prop.mapping {
                new_keys.extend(expand_expr(key, data)?);
                if let Value::Custom(value) = value {
                    new_values.extend(expand_expr(value, data)?.into_iter().map(Value::Custom));
                } else {
                    new_values.push(value.clone());
                }
            }
            replacements.push((e.name.clone(), prop.name.clone(), new_keys, new_values));
        }
    }

    for (e, prop, new_keys, new_values) in replacements {
        let e = data.enums.get_mut(&e).unwrap();
        let prop = e.properties.get_mut(&prop).unwrap();

        prop.mapping = new_keys.into_iter().zip(new_values).collect();
    }

    Ok(())
}

// Fancy, hacky regex for thrown-together parsing.
// FIXME: someone should write a proper parser
static EXPR_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("\\$\\{[^}]+}").unwrap());

fn expand_expr(expr: &str, data: &Data) -> anyhow::Result<Vec<String>> {
    let mut sliced_expr = expr;
    let mut offset = 0;

    let mut ranges = vec![];
    while let Some(m) = EXPR_REGEX.find(sliced_expr) {
        ranges.push(Range {
            start: m.start() + offset,
            end: m.end() + offset,
        });
        sliced_expr = &sliced_expr[m.end()..];
        offset += m.end() - m.start();
    }

    let mut results = vec![expr.to_owned()];
    for range in ranges {
        let value = &expr[range.start + 2..range.end - 1];

        let e = data.enums.get(value).ok_or_else(|| {
            anyhow::anyhow!(
                "no matching enum definition for expanded expression `{}`",
                value
            )
        })?;

        let mut new_results = vec![];
        for result in &results {
            let to_replace = format!("${{{}}}", value);
            for variant in &e.variants {
                let new = result.replace(&to_replace, variant);
                new_results.push(new);
            }
        }

        results = new_results;
    }

    Ok(results)
}
