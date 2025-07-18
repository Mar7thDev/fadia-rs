use std::{
    cell::Cell,
    sync::atomic::{AtomicU32, Ordering},
};

use fadia_engine::FNetworkGUID;
use tracing::warn;

use crate::{
    logic::layout::{PlayerControllerBase, PlayerState},
    net::World,
};

pub struct GameSession {
    pub max_spectators: usize,
    pub max_players: usize,
    pub max_splitscreens_per_connection: u8,
    cur_player_count: Cell<usize>,
    cur_spectator_count: Cell<usize>,
}

pub struct SessionLoginOptions {
    pub spectator_only: bool,
    pub splitscreen_count: u8,
}

impl GameSession {
    pub fn get_next_player_id(&self) -> u32 {
        // Start at 256, because 255 is special (means all team for some UT Emote stuff)
        const MIN_PLAYER_ID: u32 = 256;
        static NEXT_PLAYER_ID: AtomicU32 = AtomicU32::new(MIN_PLAYER_ID);

        NEXT_PLAYER_ID.fetch_add(1, Ordering::SeqCst)
    }

    pub fn approve_login(&self, options: SessionLoginOptions) -> Result<(), &'static str> {
        if self.at_capacity(options.spectator_only) {
            return Err("Server full.");
        }

        if options.splitscreen_count > self.max_splitscreens_per_connection {
            warn!(
                "approve_login: a maximum of {} splitscreen players are allowed",
                self.max_splitscreens_per_connection
            );
            return Err("Maximum spitscreen players");
        }

        Ok(())
    }

    pub fn register_player(
        &self,
        new_player_controller: FNetworkGUID,
        world: &mut World,
        unique_id: String,
    ) {
        if let Some(pc) =
            world.get_actor_archetype_new::<PlayerControllerBase>(new_player_controller)
        {
            if let Some(mut state) =
                world.get_actor_archetype_mut_new::<PlayerState>(pc.data().player_state.get())
            {
                let state = state.data_mut();
                state.player_id.set_value(self.get_next_player_id());
                state.unique_id.device.set_value(unique_id);

                self.cur_player_count.update(|count| count + 1);
            }
        }
    }

    pub fn at_capacity(&self, spectator: bool) -> bool {
        if spectator {
            self.cur_spectator_count.get() >= self.max_spectators
        } else {
            self.cur_player_count.get() >= self.max_players
        }
    }
}

impl Default for GameSession {
    fn default() -> Self {
        Self {
            max_spectators: 1,
            max_players: 1,
            max_splitscreens_per_connection: 1,
            cur_player_count: Cell::default(),
            cur_spectator_count: Cell::default(),
        }
    }
}
