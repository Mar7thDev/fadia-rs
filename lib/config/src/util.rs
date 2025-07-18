use std::{
    collections::HashMap,
    fs::{self, File},
};

use serde::de::DeserializeOwned;

use crate::{LoadConfigError, LoadDataError};

pub fn load_config_dir<T: DeserializeOwned>(
    base_dir: &str,
) -> Result<HashMap<String, T>, LoadDataError> {
    let mut output = HashMap::new();

    let dir = format!("assets/{base_dir}/");
    let dir = fs::read_dir(&dir).map_err(|err| LoadDataError {
        path: dir,
        err: err.into(),
    })?;

    for entry in dir {
        let entry = entry.unwrap();

        if entry.file_type().unwrap().is_dir() {
            continue;
        }

        let path = entry.path();
        let file_name = path.file_name().unwrap().to_str().unwrap();
        if !file_name.ends_with(".json") {
            continue;
        }

        let config_name = file_name.strip_suffix(".json").unwrap();

        output.insert(config_name.to_string(), load_config(base_dir, config_name)?);
    }

    Ok(output)
}

pub(crate) fn load_config<T: DeserializeOwned>(
    base_dir: &str,
    config_name: &str,
) -> Result<T, LoadDataError> {
    let file_path = format!("assets/{base_dir}/{config_name}.json");
    let mut file = match File::open(&file_path) {
        Ok(file) => file,
        Err(err) => {
            return Err(LoadDataError {
                path: file_path,
                err: err.into(),
            });
        }
    };

    let value = match serde_json::from_reader::<_, serde_json::Value>(&mut file) {
        Ok(serde_json::Value::Array(value)) => value,
        Ok(_) => {
            return Err(LoadDataError {
                path: file_path,
                err: LoadConfigError::UnexpectedRootDataType,
            });
        }
        Err(err) => {
            return Err(LoadDataError {
                path: file_path,
                err: err.into(),
            });
        }
    };

    let config_type = format!("{config_name}_C");

    let Some(data) = value.into_iter().find(|data| {
        if let serde_json::Value::Object(object) = data {
            object
                .get("Type")
                .map(|value| {
                    if let serde_json::Value::String(s) = value {
                        s == config_type.as_str()
                    } else {
                        false
                    }
                })
                .unwrap_or(false)
        } else {
            false
        }
    }) else {
        return Err(LoadDataError {
            path: file_path,
            err: LoadConfigError::MissingObject(String::from("Type"), config_type),
        });
    };

    serde_json::from_value(data).map_err(|err| LoadDataError {
        path: file_path,
        err: err.into(),
    })
}
