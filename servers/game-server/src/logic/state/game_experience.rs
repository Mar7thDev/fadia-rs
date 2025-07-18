use fadia_codegen::{RepLayout, dummy_rpc_handler};
use fadia_engine::replication::property::PropertyObject;

use crate::logic::ObjectLayout;

#[derive(Debug, RepLayout)]
#[dummy_rpc_handler]
pub struct GameExperienceComponent {
    #[rep(handle = 3)]
    pub current_game_experience: PropertyObject,
}

impl ObjectLayout for GameExperienceComponent {
    fn on_channel_open(&self, channel: &mut crate::net::ActorChannel, world: &crate::net::World) {
        let game_experience = self.current_game_experience.get();

        if game_experience.is_valid() {
            channel.export_net_guid(world.export_guid(game_experience));
        }
    }
}
