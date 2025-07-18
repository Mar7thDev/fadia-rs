use std::collections::HashMap;

use serde::Deserialize;

use crate::{util, LoadDataError};

use super::ClassDefinition;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct PlayerCharacterWeaponConfig {
    #[serde(deserialize_with = "super::deserialize_class_definition")]
    pub class: ClassDefinition,
    pub name: String,
}

pub fn load_player_character_weapon_configs()
-> Result<HashMap<String, PlayerCharacterWeaponConfig>, LoadDataError> {
    const WEAPON_CONFIG_DIR: &str = "Blueprints/Weapon/Player";

    let map = util::load_config_dir(WEAPON_CONFIG_DIR)?
        .into_iter()
        .map(|(key, value)| (key.split('_').next_back().unwrap().to_string(), value))
        .collect();

    Ok(map)
}
