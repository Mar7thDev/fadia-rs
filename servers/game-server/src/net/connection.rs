use std::{
    borrow::Cow,
    cell::OnceCell,
    collections::{BTreeMap, VecDeque},
    io::{self, Cursor},
};

use bitstream_io::{BitRead, BitWrite, BitWriter, LittleEndian};
use fadia_engine::{
    FNetworkGUID,
    util::{self, FName, ReadBitsExt, WriteBitsExt},
};
use fadia_engine::{
    net::{Bunch, FNetPacketNotify},
    util::InBitReader,
};
use tracing::{debug, error};

use crate::logic::{actor::NetPlayerIndex, mode::SessionLoginOptions, replication::InRPC};

use super::{
    SessionID,
    channel::{ActorChannel, Challenge, Channel, ControlChannel, NAME_CONTROL_CHANNEL, Welcome},
    udp_server::PacketSender,
    world::World,
};

#[derive(thiserror::Error, Debug)]
pub enum ReceivePacketError {
    #[error("failed to read PacketHeader: {0}")]
    HeaderRead(io::Error),
    #[error("read_packet_info failed: {0}")]
    ReadPacketInfo(io::Error),
    #[error("dispatch_packet failed: {0}")]
    DispatchPacket(io::Error),
}

pub struct NetConnection {
    pub session_id: SessionID,
    pub control_channel: Channel<ControlChannel>,
    pub actor_channels: BTreeMap<u32, Channel<ActorChannel>>,
    pub player_controller: Option<FNetworkGUID>,
    pub current_net_speed: u32,
    pub unique_id: OnceCell<String>,
    player_index: NetPlayerIndex,
    output: PacketSender,
    packet_notify: FNetPacketNotify,
    init_in_reliable: u16,
    init_out_reliable: u16,
    send_queue: VecDeque<(Vec<u8>, usize)>,
}

impl NetConnection {
    pub fn new(
        session_id: SessionID,
        player_index: NetPlayerIndex,
        (out_seq, in_seq): (u16, u16),
        output: PacketSender,
    ) -> Self {
        let mut connection = Self {
            session_id,
            player_index,
            output,
            unique_id: OnceCell::new(),
            packet_notify: FNetPacketNotify::default(),
            init_in_reliable: 0,
            init_out_reliable: 0,
            control_channel: Channel::new(
                0,
                in_seq & 1023,
                out_seq & 1023,
                NAME_CONTROL_CHANNEL.clone(),
                ControlChannel::default(),
            ),
            actor_channels: BTreeMap::new(),
            send_queue: VecDeque::new(),
            player_controller: None,
            current_net_speed: 0,
        };

        connection.init_sequence(out_seq, in_seq);
        connection
    }

    pub fn create_actor_channel(
        &mut self,
        at_index: u32,
        actor_guid: FNetworkGUID,
        name: FName,
    ) -> Option<&mut Channel<ActorChannel>> {
        if self.actor_channels.contains_key(&at_index) {
            error!("failed to create channel {name:?} at {at_index}. Index is busy.");
            return None;
        }

        self.actor_channels.insert(
            at_index,
            Channel::new(
                at_index,
                self.init_in_reliable,
                self.init_out_reliable,
                name,
                ActorChannel::new(actor_guid),
            ),
        );

        Some(self.actor_channels.get_mut(&at_index).unwrap())
    }

    pub fn get_channel_actor(&self, index: u32) -> Option<FNetworkGUID> {
        self.actor_channels
            .get(&index)
            .map(|channel| channel.channel_impl.actor_guid)
    }

    pub fn net_player_index(&self) -> NetPlayerIndex {
        self.player_index
    }

    pub fn next_rpc(&mut self) -> Option<(u32, FNetworkGUID, InRPC)> {
        for (&index, chan) in self.actor_channels.iter_mut() {
            if let Some((guid, rpc)) = chan.channel_impl.next_rpc() {
                return Some((index, guid, rpc));
            }
        }

        None
    }

    pub fn login_options(&self) -> SessionLoginOptions {
        SessionLoginOptions {
            spectator_only: false,
            splitscreen_count: 0,
        }
    }

    pub fn received_packet(
        &mut self,
        world: &mut World,
        r: &mut InBitReader,
        bits_count: usize,
        dispatch_packet: bool,
    ) -> Result<(), ReceivePacketError> {
        let header = self
            .packet_notify
            .read_header(r)
            .map_err(ReceivePacketError::HeaderRead)?;

        let packet_sequence_delta = self.packet_notify.get_sequence_delta(&header);
        if packet_sequence_delta > 0 {
            if packet_sequence_delta > 0 {
                self.packet_notify.process_received_acks(&header);
                self.packet_notify
                    .internal_update(&header, packet_sequence_delta);
            }

            if self.packet_notify.is_waiting_for_sequence_history_flush() {
                // This thing is noisy and not really important,
                // should implement someday but not now
            }
        }

        let packet_id = header.packed_header.get_seq();

        self.packet_notify.ack_seq(packet_id, true);
        self.read_packet_info(r)
            .map_err(ReceivePacketError::ReadPacketInfo)?;

        if dispatch_packet {
            self.dispatch_packet(world, r, bits_count as u64, packet_id)
                .map_err(ReceivePacketError::DispatchPacket)?;
        }

        Ok(())
    }

    fn dispatch_packet(
        &mut self,
        world: &mut World,
        r: &mut InBitReader,
        size_in_bits: u64,
        packet_id: u16,
    ) -> io::Result<()> {
        while r.position_in_bits().unwrap() < size_in_bits {
            let bunch = Bunch::decode(r, packet_id).unwrap();
            let data = r.read_bits(bunch.bunch_data_bits)?.into_boxed_slice();

            if bunch.reliable {
                if bunch.ch_index == 0 {
                    self.control_channel
                        .received_raw_bunch(world, bunch, data)?;

                    for message in
                        std::mem::take(&mut self.control_channel.channel_impl.received_messages)
                    {
                        world.notify_control_message(self, message)?;
                    }
                } else if let Some(channel) = self.actor_channels.get_mut(&bunch.ch_index) {
                    channel.received_raw_bunch(world, bunch, data)?;
                }
            }
        }

        Ok(())
    }

    fn read_packet_info<R: BitRead>(&self, r: &mut R) -> io::Result<()> {
        let has_packet_info_payload = r.read_bit()?;
        if has_packet_info_payload {
            let _packet_jitter_clock_time_ms: u16 = r.read(10)?;
            let _has_server_frame_time = r.read_bit()?;
            // frame time is only encoded in server to client packets
        }

        Ok(())
    }

    pub fn init_sequence(&mut self, out_sequence: u16, in_sequence: u16) {
        self.init_in_reliable = in_sequence & (1024 - 1);
        self.init_out_reliable = out_sequence & (1024 - 1);
        debug!(
            "init in reliable: {} init out reliable: {}",
            self.init_in_reliable, self.init_out_reliable
        );
        self.packet_notify.init(in_sequence, out_sequence);
    }

    pub fn send_challenge_control_message(&mut self) -> io::Result<()> {
        Challenge::send(self, String::from("8B69DF87"))
    }

    pub fn send_welcome_control_message(
        &mut self,
        map: Cow<'static, str>,
        game_name: Cow<'static, str>,
        redirect_url: Cow<'static, str>,
    ) -> io::Result<()> {
        Welcome::send(self, map, game_name, redirect_url)
    }

    pub fn send_control_channel_reliable_bunch(
        &mut self,
        buffer_terminated: &[u8],
    ) -> io::Result<()> {
        const CONTROL_CHANNEL_INDEX: u32 = 0;

        let bits = util::get_bits_from_terminated_stream(buffer_terminated).unwrap();
        let sequence = self.control_channel.next_out_reliable();

        let bunch = Bunch {
            control: false,
            open: false,
            close: false,
            close_reason: 0,
            reliable: true,
            partial: false,
            partial_initial: false,
            partial_final: false,
            has_package_map_exports: false,
            has_must_be_mapped_guids: false,
            is_replication_paused: false,
            ch_index: CONTROL_CHANNEL_INDEX,
            ch_sequence: sequence,
            ch_name: Some(FName::Hardcoded(255)),
            bunch_data_bits: bits as usize,
            network_exports_data: None,
        };

        self.send_raw_bunches(&[(bunch, buffer_terminated)])
    }

    pub fn send_raw_bunches(&mut self, bunches: &[(Bunch, &[u8])]) -> io::Result<()> {
        let mut buf = (Vec::new(), 0);
        let mut w = BitWriter::endian(Cursor::new(&mut buf.0), LittleEndian);
        for (bunch, data) in bunches {
            bunch.encode(&mut w)?;

            if bunch.has_package_map_exports {
                let (exports_buf, bit_size) = bunch.network_exports_data.as_ref().unwrap();
                w.write_bits(exports_buf, *bit_size as usize)?;
            }

            if !data.is_empty() {
                w.write_bits(
                    data,
                    util::get_bits_from_terminated_stream(data).unwrap() as usize,
                )?;
            }
        }
        w.write_bit(true)?; // termination bit
        w.byte_align()?;

        buf.1 = util::get_bits_from_terminated_stream(&buf.0).unwrap();
        self.send_queue.push_back(buf);
        Ok(())
    }

    pub fn send_raw_bunch(&mut self, bunch: Bunch, data: &[u8]) -> io::Result<()> {
        let mut buf = (Vec::new(), 0);
        let mut w = BitWriter::endian(Cursor::new(&mut buf.0), LittleEndian);

        bunch.encode(&mut w)?;

        if bunch.has_package_map_exports {
            let (exports_buf, bit_size) = bunch.network_exports_data.as_ref().unwrap();
            w.write_bits(exports_buf, *bit_size as usize)?;
        }

        if !data.is_empty() {
            // bunch_data_bits usage here is valid
            // package_map_exports bunches are always separate from data bunches
            w.write_bits(data, bunch.bunch_data_bits)?;
        }

        w.write_bit(true)?; // termination bit
        w.byte_align()?;

        buf.1 = util::get_bits_from_terminated_stream(&buf.0).unwrap();
        self.send_queue.push_back(buf);

        Ok(())
    }

    pub fn has_awaiting_send_packets(&self) -> bool {
        !self.send_queue.is_empty()
    }

    pub fn flush_net(&mut self) -> io::Result<()> {
        loop {
            let mut buf = Vec::new();
            let mut w = BitWriter::endian(Cursor::new(&mut buf), LittleEndian);
            w.write(2, self.session_id.session_id)?;
            w.write(3, self.session_id.client_id)?;
            w.write_bit(false)?;
            self.flush_outgoing_packet(&mut w).unwrap();
            w.write_bit(true)?;
            w.byte_align()?;

            self.output.send(buf.into_boxed_slice());

            if !self.has_awaiting_send_packets() {
                break Ok(());
            }
        }
    }

    fn flush_outgoing_packet<W: BitWrite>(&mut self, w: &mut W) -> io::Result<()> {
        self.packet_notify.write_header(w, false)?;
        self.packet_notify.commit_and_increment_seq();

        // PacketInfo
        w.write_bit(true)?;
        w.write(10, 0)?; // jitter
        w.write_bit(true)?;
        w.write(8, 0)?; // frame time

        if let Some((data, length)) = self.send_queue.pop_front() {
            w.write_bits(&data, length)?;
        }

        w.write_bit(true)?; // termination

        Ok(())
    }
}
