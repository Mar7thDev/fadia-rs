use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct FunctionUnlockData {
    #[serde(rename = "ID")]
    pub id: String,
}
