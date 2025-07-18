use fadia_codegen::replicated_enum;
use fadia_engine::{
    FNetworkGUID,
    rotator::FRotator,
    util::{OutBitWriter, WritePrimitivesExt},
    vector::FVector3d,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetPlayerIndex(pub u8);

#[derive(Debug)]
pub struct Actor {
    pub self_guid: FNetworkGUID,
    pub archetype_guid: FNetworkGUID,
    pub position: FVector3d,
    pub rotation: FRotator,
    pub controls_player: Option<NetPlayerIndex>,
}

#[allow(unused)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[replicated_enum]
pub enum NetRole {
    None = 0,
    SimulatedProxy = 1,
    AutonomousProxy = 2,
    Authority = 3,
    Max = 4,
}

impl Actor {
    pub fn new(guid: FNetworkGUID, archetype: FNetworkGUID) -> Self {
        Self {
            self_guid: guid,
            archetype_guid: archetype,
            position: FVector3d::default(),
            rotation: FRotator::default(),
            controls_player: None,
        }
    }

    pub fn set_controlled_player(&mut self, index: NetPlayerIndex) {
        self.controls_player = Some(index);
    }

    pub fn on_channel_opened(&self, out: &mut OutBitWriter) {
        if let Some(NetPlayerIndex(index)) = self.controls_player.as_ref() {
            out.write_u8(*index).unwrap();
        }
    }
}
