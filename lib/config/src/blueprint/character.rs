use std::collections::HashMap;

use num_enum::IntoPrimitive;
use serde::Deserialize;

use crate::{util, LoadDataError};

use super::{ClassDefinition, ClassReference};

const PLAYER_CHARACTER_CONFIG_DIR: &str = "Blueprints/Character/Player";

#[derive(Deserialize, IntoPrimitive, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ESkillInputIDType {
    #[serde(rename = "ESkillInputIDType::InputID_None")]
    None = 0,
    #[serde(rename = "ESkillInputIDType::InputID_Melee")]
    Melee = 1,
    #[serde(rename = "ESkillInputIDType::InputID_Evade")]
    Evade = 2,
    #[serde(rename = "ESkillInputIDType::InputID_Skill")]
    Skill = 3,
    #[serde(rename = "ESkillInputIDType::255")]
    Passive = 255,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct CharacterAbilityEntry {
    pub ability_class: ClassReference,
    #[serde(rename = "InputID")]
    pub input_id: ESkillInputIDType,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct PlayerCharacterProperties {
    #[serde(default)]
    pub granted_abilities: Vec<CharacterAbilityEntry>,
    #[serde(default)]
    pub passive_abilities: Vec<CharacterAbilityEntry>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct PlayerCharacterConfig {
    pub name: String,
    #[serde(deserialize_with = "super::deserialize_class_definition")]
    pub class: ClassDefinition,
    pub properties: PlayerCharacterProperties,
}

pub fn load_player_character_configs()
-> Result<HashMap<String, PlayerCharacterConfig>, LoadDataError> {
    util::load_config_dir(PLAYER_CHARACTER_CONFIG_DIR)
}
