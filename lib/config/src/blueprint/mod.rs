use serde::{Deserialize, Deserializer};

mod ability;
mod character;
mod weapon;

pub use ability::*;
pub use character::*;
pub use weapon::*;

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "PascalCase")]
pub struct ClassReference {
    #[serde(deserialize_with = "deserialize_class_asset_path")]
    pub asset_path_name: (String, String),
    pub sub_path_string: String,
}

#[derive(Debug, Default)]
pub struct ClassDefinition {
    pub ty: String,
    pub path: String,
    pub name: String,
}

impl ClassReference {
    pub fn is_empty(&self) -> bool {
        self.asset_path_name.0.is_empty() || self.asset_path_name.1.is_empty()
    }
}

pub(crate) fn deserialize_class_asset_path<'de, D>(de: D) -> Result<(String, String), D::Error>
where
    D: Deserializer<'de>,
{
    let path = String::deserialize(de)?;

    if !path.is_empty() {
        let mut path = path.split('.');

        Ok((
            path.next().unwrap().to_string(),
            path.next().unwrap().to_string(),
        ))
    } else {
        Ok(Default::default())
    }
}

pub(crate) fn deserialize_class_definition<'de, D>(de: D) -> Result<ClassDefinition, D::Error>
where
    D: Deserializer<'de>,
{
    let path = String::deserialize(de)?;

    if !path.is_empty() {
        let mut path = path.split('\'');
        let ty = path.next().unwrap().to_string();

        let path = path.next().unwrap().to_string();
        let mut path = path.split('.');

        Ok(ClassDefinition {
            ty,
            path: path.next().unwrap().to_string(),
            name: path.next().unwrap().to_string(),
        })
    } else {
        Ok(Default::default())
    }
}
