use std::collections::HashMap;

use fadia_config::{
    LoadDataError,
    blueprint::{
        ClassReference, PlayerCharacterAbilityConfig, PlayerCharacterConfig,
        PlayerCharacterWeaponConfig,
    },
    dataset::DataAssetSet,
};

pub struct GameAssets {
    pub data_asset_set: DataAssetSet,
    player_character_configs: HashMap<String, PlayerCharacterConfig>,
    player_character_abilities: HashMap<ClassReference, PlayerCharacterAbilityConfig>,
    player_character_weapons: HashMap<String, PlayerCharacterWeaponConfig>,
}

#[derive(thiserror::Error, Debug)]
pub enum AssetsLoadingError {
    #[error("failed to load DataAssetSet: {0}")]
    DataAssetSet(LoadDataError),
    #[error("failed to load player character configs: {0}")]
    PlayerCharacterConfigs(LoadDataError),
    #[error("failed to load player character ability configs: {0}")]
    PlayerCharacterAbilityConfigs(LoadDataError),
    #[error("failed to load player character weapon configs: {0}")]
    PlayerCharacterWeaponConfigs(LoadDataError),
}

impl GameAssets {
    pub fn load() -> Result<Self, AssetsLoadingError> {
        let data_asset_set = DataAssetSet::load().map_err(AssetsLoadingError::DataAssetSet)?;

        let player_character_configs = fadia_config::blueprint::load_player_character_configs()
            .map_err(AssetsLoadingError::PlayerCharacterConfigs)?;

        let player_character_abilities = Self::load_abilities(player_character_configs.values())
            .map_err(AssetsLoadingError::PlayerCharacterAbilityConfigs)?;

        let player_character_weapons =
            fadia_config::blueprint::load_player_character_weapon_configs()
                .map_err(AssetsLoadingError::PlayerCharacterWeaponConfigs)?;

        Ok(Self {
            data_asset_set,
            player_character_configs,
            player_character_abilities,
            player_character_weapons,
        })
    }

    fn load_abilities<'cfg>(
        chars: impl Iterator<Item = &'cfg PlayerCharacterConfig>,
    ) -> Result<HashMap<ClassReference, PlayerCharacterAbilityConfig>, LoadDataError> {
        let mut output = HashMap::new();

        for config in chars {
            for ability in config
                .properties
                .granted_abilities
                .iter()
                .chain(config.properties.passive_abilities.iter())
            {
                if !ability.ability_class.is_empty() {
                    let ability_config = fadia_config::blueprint::load_player_character_ability(
                        &ability.ability_class,
                    )?;

                    output.insert(ability.ability_class.clone(), ability_config);
                }
            }
        }

        Ok(output)
    }

    pub fn get_player_character_config(&self, name: &str) -> Option<&PlayerCharacterConfig> {
        self.player_character_configs.get(name)
    }

    pub fn get_player_ability_config(
        &self,
        class: &ClassReference,
    ) -> Option<&PlayerCharacterAbilityConfig> {
        self.player_character_abilities.get(class)
    }

    pub fn get_weapon_for_character(
        &self,
        character_config_name: &str,
    ) -> Option<&PlayerCharacterWeaponConfig> {
        let normalized_char_name = character_config_name
            .split('_')
            .next_back()
            .unwrap()
            .chars()
            .enumerate()
            .map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c })
            .collect::<String>();

        self.player_character_weapons.get(&normalized_char_name)
    }
}
