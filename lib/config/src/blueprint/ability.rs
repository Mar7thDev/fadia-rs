use std::path::Path;

use serde::Deserialize;

use crate::{util, LoadDataError};

use super::ClassReference;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct PlayerCharacterAbilityConfig {
    pub name: String,
}

pub fn load_player_character_ability(
    ClassReference {
        asset_path_name: (outer, inner),
        ..
    }: &ClassReference,
) -> Result<PlayerCharacterAbilityConfig, LoadDataError> {
    let base_dir = Path::new(outer.strip_prefix("/Game/").unwrap());

    let mut components = base_dir.components();
    components.next_back();
    let base_dir = components.as_path();

    util::load_config(
        base_dir.to_str().unwrap(),
        inner.strip_suffix("_C").unwrap(),
    )
}
