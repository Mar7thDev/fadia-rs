use std::{collections::HashMap, fs::File};

use serde::{Deserialize, de::DeserializeOwned};

use crate::LoadDataError;

mod avatar;
mod function_unlock;

pub use avatar::*;
pub use function_unlock::*;

const DATASET_BASE_PATH: &str = "DataAssets/DataAssetSet";
const DT_BASE_PATH: &str = "DataTable";

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct DataTable<Data> {
    pub rows: HashMap<String, Data>,
}

pub struct DataAssetSet {
    pub function_unlock_table: DataTable<FunctionUnlockData>,
    pub avatar_data_table: DataTable<AvatarData>,
}

impl DataAssetSet {
    pub fn load() -> Result<Self, LoadDataError> {
        Ok(Self {
            function_unlock_table: DataTable::load_from_file(&format!(
                "assets/{DATASET_BASE_PATH}/FunctionUnlock/FunctionUnlockDataTable.json"
            ))?,
            avatar_data_table: DataTable::load_from_file(&format!(
                "assets/{DT_BASE_PATH}/Avatar/AvatarDataTable.json"
            ))?,
        })
    }
}

impl<T: DeserializeOwned> DataTable<T> {
    fn load_from_file(path: &str) -> Result<Self, LoadDataError> {
        let mut file = File::open(path).map_err(|err| LoadDataError {
            path: path.to_string(),
            err: err.into(),
        })?;

        serde_json::from_reader::<_, [_; 1]>(&mut file)
            .map_err(|err| LoadDataError {
                path: path.to_string(),
                err: err.into(),
            })
            .map(|[data]| data)
    }
}
