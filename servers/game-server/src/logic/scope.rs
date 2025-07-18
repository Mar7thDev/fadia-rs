use std::collections::HashMap;

use crate::{
    assets::GameAssets,
    config::GameplayGlobals,
    net::{ConnectParams, NetConnection, SessionID, World},
};

use super::{actor::NetPlayerIndex, mode::HTGameMode};

#[derive(Default)]
pub struct LogicScopeManager {
    pub session_scopes: HashMap<SessionID, ScopeID>,
    pub scopes: HashMap<ScopeID, LogicScope>,
    pub scope_counter: u64,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct ScopeID(u64);

pub struct LogicScope {
    pub world: World,
    pub connections: HashMap<SessionID, NetConnection>,
}

impl LogicScopeManager {
    pub fn create_scope(
        &mut self,
        params: ConnectParams,
        globals: &'static GameplayGlobals,
        assets: &'static GameAssets,
    ) -> ScopeID {
        self.scope_counter += 1;
        let scope_id = ScopeID(self.scope_counter);

        let world = World::new::<HTGameMode>(assets, globals);

        let connection = NetConnection::new(
            params.session_id,
            NetPlayerIndex(0),
            (params.server_seq, params.client_seq),
            params.output,
        );

        self.session_scopes.insert(params.session_id, scope_id);
        self.scopes.insert(
            scope_id,
            LogicScope {
                world,
                connections: HashMap::from([(params.session_id, connection)]),
            },
        );

        scope_id
    }

    pub fn get_scope_for_session(&mut self, session_id: SessionID) -> Option<&mut LogicScope> {
        self.session_scopes
            .get(&session_id)
            .and_then(|scope_id| self.scopes.get_mut(scope_id))
    }
}
