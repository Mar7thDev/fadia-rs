use std::io;

pub mod blueprint;
pub mod dataset;

mod util;

#[derive(thiserror::Error, Debug)]
#[error("failed to load {path}: {err}")]
pub struct LoadDataError {
    pub path: String,
    pub err: LoadConfigError,
}

#[derive(thiserror::Error, Debug)]
pub enum LoadConfigError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("failed to parse the json structure: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unexpected root data type")]
    UnexpectedRootDataType,
    #[error("config is missing an object with {0}:{1}")]
    MissingObject(String, String),
}
