use std::{
    collections::{HashSet, VecDeque},
    io::{self, Cursor},
};

use bitstream_io::{BitRead, BitWrite};
use fadia_engine::{
    FNetworkGUID,
    net::Bunch,
    package_map::{ExportFlags, FNetFieldExport, FNetFieldExportGroup},
    util::{
        self, InBitReader, OutBitWriter, PackedBitReadExt, PackedBitWriteExt, ReadBitsExt,
        WritePrimitivesExt, quantized::QuantizedWriteExt,
    },
};
use tracing::{error, warn};

use crate::{
    logic::{
        Object,
        actor::Actor,
        replication::{self, InRPC, ReceiveClientDataError},
    },
    net::{World, world::FNetFieldExportChain},
};

use super::ChannelImpl;

pub struct ActorChannel {
    pub actor_guid: FNetworkGUID,
    pub pending_export_group: Option<FNetFieldExportGroup>,
    pub exported_guids: HashSet<FNetworkGUID>,
    pub must_be_mapped_guids: HashSet<FNetworkGUID>,
    incoming_rpc_queue: VecDeque<(FNetworkGUID, InRPC)>,
    queued_bunches: Vec<(Bunch, Box<[u8]>)>,
    open_bunch_sent: bool,
    spawn_bunch_sent: bool,
    initial_bunches_sent_for_guids: HashSet<FNetworkGUID>,
}

impl ActorChannel {
    pub fn new(actor_guid: FNetworkGUID) -> Self {
        Self {
            actor_guid,
            pending_export_group: None,
            exported_guids: HashSet::new(),
            must_be_mapped_guids: HashSet::new(),
            incoming_rpc_queue: VecDeque::new(),
            queued_bunches: Vec::new(),
            open_bunch_sent: false,
            spawn_bunch_sent: false,
            initial_bunches_sent_for_guids: HashSet::new(),
        }
    }

    pub fn next_rpc(&mut self) -> Option<(FNetworkGUID, InRPC)> {
        self.incoming_rpc_queue.pop_front()
    }

    pub fn export_net_guid(&mut self, export_chain: FNetFieldExportChain) {
        let export_group = self.pending_export_group.get_or_insert_default();

        let (encoded_export_guid, _, no_load) = export_chain.0.first().copied().unwrap();
        let mut outers = export_chain.0.iter().skip(1);
        let mut inners = export_chain.0.iter();

        if !no_load {
            self.must_be_mapped_guids.insert(encoded_export_guid);
        }

        while let (outer, Some((inner, path, no_load))) = (outers.next(), inners.next()) {
            export_group.exported_fields.insert(
                *inner,
                FNetFieldExport {
                    path: path.to_string(),
                    should_encode: *inner == encoded_export_guid,
                    export_flags: {
                        let mut flags = ExportFlags::default();

                        if !path.is_empty() {
                            flags = flags.set_has_path();

                            if *no_load {
                                flags = flags.set_no_load();
                            }
                        } else {
                            flags = flags.set_no_load();
                        }

                        flags
                    },
                    outer_guid: outer
                        .map(|(guid, _, _)| *guid)
                        .unwrap_or(FNetworkGUID::INVALID),
                    network_checksum: 0,
                },
            );

            self.exported_guids.insert(*inner);
        }
    }

    pub fn tick(&mut self, world: &mut World) {
        if !world.actors.contains_key(&self.actor_guid) {
            error!(
                "ActorChannel::tick: actor is gone! GUID: {:?}",
                self.actor_guid
            );
            return;
        }

        if self.should_send_bunch(world) {
            let mut bunch_data = Vec::new();
            let mut out = OutBitWriter::new(&mut bunch_data);

            let has_must_be_mapped_guids = self.flush_pending_mapped_guids(&mut out);

            if !self.spawn_bunch_sent {
                self.spawn_bunch_sent = true;
                self.prepare_spawn_bunch(&mut out, world);
            }

            self.write_objects(&mut out, [self.actor_guid].into(), world);

            out.write_bit(true).unwrap(); // termination bit
            out.byte_align().unwrap();

            let bunch_data_bits = util::get_bits_from_terminated_stream(&bunch_data).unwrap();

            self.queued_bunches.push((
                Bunch {
                    has_must_be_mapped_guids,
                    bunch_data_bits,
                    ..Default::default()
                },
                bunch_data.into_boxed_slice(),
            ));
        }
    }

    fn should_send_bunch(&self, world: &World) -> bool {
        !self.spawn_bunch_sent
            || world.any_sub_object_has_queued_rpc(self.actor_guid)
            || world.any_sub_object_has_changes(self.actor_guid)
    }

    fn prepare_spawn_bunch(&mut self, writer: &mut OutBitWriter, world: &mut World) {
        let actor = world.actors.get(&self.actor_guid).unwrap();
        serialize_new_actor(writer, actor).unwrap();
    }

    fn write_objects(
        &mut self,
        out: &mut OutBitWriter,
        objects: HashSet<FNetworkGUID>,
        world: &mut World,
    ) {
        let actor_archetype_guid = world.actors.get(&self.actor_guid).unwrap().archetype_guid;

        objects.iter().for_each(|&guid| {
            let object = world.objects.get_mut(&guid).unwrap();
            if (self.actor_guid != guid || guid == actor_archetype_guid)
                && (!object.rep_layout.is_empty()
                    || !object.queued_rpcs.is_empty()
                    || guid.is_dynamic())
            {
                let is_spawn_bunch = self.initial_bunches_sent_for_guids.insert(guid);
                self.write_object_data(
                    out,
                    guid,
                    object,
                    guid == actor_archetype_guid,
                    is_spawn_bunch,
                );
            }

            self.write_objects(out, object.sub_objects.clone(), world);
        });
    }

    fn write_object_data(
        &mut self,
        writer: &mut OutBitWriter,
        guid: FNetworkGUID,
        object: &mut Object,
        is_actor: bool,
        send_full_data: bool,
    ) {
        replication::serialize_object(writer, guid, object, is_actor, send_full_data).unwrap();

        // Clear data that is already serialized
        object.rep_layout.acknowledge_changes();
        object.queued_rpcs.clear();
    }

    fn flush_pending_net_exports(&mut self, output: &mut Vec<(Bunch, Box<[u8]>)>) {
        if let Some(mut export_group) = self.pending_export_group.take() {
            while let Some(sub_group) = export_group.split_to_fit(Bunch::MAX_DATA_BITS) {
                output.push((make_net_field_export_bunch(sub_group), Box::from([])));
            }
        }
    }

    fn flush_pending_mapped_guids(&mut self, out: &mut OutBitWriter) -> bool {
        if !self.must_be_mapped_guids.is_empty() {
            let must_be_mapped_guids = std::mem::take(&mut self.must_be_mapped_guids);

            out.write_u16(must_be_mapped_guids.len() as u16).unwrap();
            for FNetworkGUID(guid) in must_be_mapped_guids {
                out.write_packed_int(guid).unwrap();
            }

            true
        } else {
            false
        }
    }

    fn prepare_bunch_group(&mut self, bunches: &mut [(Bunch, Box<[u8]>)]) {
        if !bunches.is_empty() {
            let num_bunches = bunches.len();
            let is_multiple = num_bunches > 1;

            for (i, (bunch, _)) in bunches.iter_mut().enumerate() {
                bunch.reliable = true; // all bunches are reliable for now
                bunch.partial = is_multiple;
                bunch.partial_initial = is_multiple && i == 0;
                bunch.partial_final = is_multiple && i == num_bunches - 1;

                if !self.open_bunch_sent && i == 0 {
                    bunch.control = true;
                    bunch.open = true;
                }
            }

            self.open_bunch_sent = true;
        }
    }

    fn read_content_block_payload<R: BitRead>(
        &self,
        bunch: &Bunch,
        r: &mut R,
    ) -> io::Result<(FNetworkGUID, bool, Box<[u8]>, usize)> {
        let (object_guid, has_rep_layout) = self.read_content_block_header(r)?;

        if !object_guid.is_valid() {
            error!(
                "received invalid NetworkGUID from client, channel: {}",
                bunch.ch_index
            );
            return Ok((object_guid, has_rep_layout, Box::from([]), 0));
        }

        let num_payload_bits = r.read_packed_int()? as usize;
        let payload = r.read_bits(num_payload_bits)?.into_boxed_slice();

        Ok((object_guid, has_rep_layout, payload, num_payload_bits))
    }

    fn read_content_block_header<R: BitRead>(&self, r: &mut R) -> io::Result<(FNetworkGUID, bool)> {
        let has_rep_layout = r.read_bit()?;
        let is_actor = r.read_bit()?;

        if is_actor {
            Ok((self.actor_guid, has_rep_layout))
        } else {
            Ok((FNetworkGUID(r.read_packed_int()?), has_rep_layout))
        }
    }
}

impl ChannelImpl for ActorChannel {
    fn received_bunch(
        &mut self,
        world: &mut super::World,
        bunch: &Bunch,
        r: &mut InBitReader,
    ) -> std::io::Result<()> {
        if bunch.has_must_be_mapped_guids {
            error!(
                "ActorChannel::received_bunch: client attempted to set has_must_be_mapped_guids, channel: {}",
                bunch.ch_index
            );
            return Ok(());
        }

        while (r.position_in_bits().unwrap() as usize) < bunch.bunch_data_bits - 1 {
            let Ok((guid, has_rep_layout, payload, size_in_bits)) =
                self.read_content_block_payload(bunch, r)
            else {
                break;
            };

            // Server shouldn't receive properties.
            if has_rep_layout {
                error!(
                    "server received RepLayout properties for object {guid:?} at channel {}",
                    bunch.ch_index
                );
                continue;
            }

            let mut r = InBitReader::new(Cursor::new(payload.as_ref()));

            let guid = if guid == self.actor_guid {
                world.actors.get(&self.actor_guid).unwrap().archetype_guid
            } else {
                guid
            };

            let Some(object) = world.objects.get(&guid) else {
                error!("received RPCs for non-existent object, GUID: {guid:?}");
                continue;
            };

            match replication::receive_client_data(object, &mut r, size_in_bits) {
                Ok(rpcs) => self
                    .incoming_rpc_queue
                    .extend(rpcs.into_iter().map(|rpc| (guid, rpc))),
                Err(ReceiveClientDataError::NoMaxRepIndex) => {
                    warn!(
                        "max_rep_index is not defined for object with GUID {guid:?}, channel: {}, path: {}",
                        bunch.ch_index,
                        world.net_guid_cache.get_path_name_by_guid(guid)
                    );
                    continue;
                }
                Err(ReceiveClientDataError::Io(err)) => return Err(err),
            }
        }

        Ok(())
    }

    fn remove_queued_bunches(&mut self) -> Vec<(Bunch, Box<[u8]>)> {
        let mut output = Vec::new();

        self.flush_pending_net_exports(&mut output);

        for (bunch, data) in std::mem::take(&mut self.queued_bunches) {
            if bunch.bunch_data_bits > Bunch::MAX_DATA_BITS {
                for (data, bits) in super::split_bunch_data(data, bunch.bunch_data_bits) {
                    output.push((
                        Bunch {
                            bunch_data_bits: bits,
                            ..bunch.clone()
                        },
                        data,
                    ));
                }
            } else {
                output.push((bunch, data));
            }
        }

        self.prepare_bunch_group(&mut output);
        output
    }
}

fn serialize_new_actor(w: &mut OutBitWriter, actor: &Actor) -> std::io::Result<()> {
    w.write_packed_int(actor.self_guid.0)?;
    if actor.self_guid.is_dynamic() {
        w.write_packed_int(actor.archetype_guid.0)?;
        w.write_packed_int(3)?; // Level GUID

        let pos_serialized = !actor.position.is_zero();
        w.write_bit(pos_serialized)?;
        if pos_serialized {
            w.write_bit(true)?; // bPosQuantized, TODO: conditional vector quantization
            w.write_packed_vector(&actor.position, 10)?;
        }

        let rot_serialized = actor.rotation.should_serialize();
        w.write_bit(rot_serialized)?;
        if rot_serialized {
            actor.rotation.net_serialize(w)?;
        }

        w.write_bit(false)?; // bScaleSerialized
        w.write_bit(false)?; // bVelocitySerialized
    }

    actor.on_channel_opened(w);

    Ok(())
}

fn make_net_field_export_bunch(export_group: FNetFieldExportGroup) -> Bunch {
    let mut bunch = Bunch {
        has_package_map_exports: true,
        ..Default::default()
    };

    bunch.add_network_exports(&export_group);
    bunch.refresh_data_bits_num(&[]);

    bunch
}
