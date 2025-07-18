use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct AvatarData {
    #[serde(rename = "bIsDefault")]
    pub is_default: bool,
    #[serde(rename = "bAutoUnlock")]
    pub auto_unlock: bool,
    #[serde(rename = "bNeedMatchSex")]
    pub need_match_sex: bool,
    pub role_sex: ERoleSex,
}

#[derive(Deserialize, Debug)]
pub enum ERoleSex {
    #[serde(rename = "ERoleSex::Male")]
    Male,
    #[serde(rename = "ERoleSex::Female")]
    Female
}
