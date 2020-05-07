use crate::model::{Model, ModelFile, Type, VecOrOne};
use anyhow::Context;
use heck::CamelCase;
use itertools::Either;
use std::collections::BTreeMap;

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
    Ok(data)
}

fn add_to_data<'a>(file: &str, file_name: &str, data: &mut Data) -> anyhow::Result<()> {
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

#[derive(Debug)]
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
