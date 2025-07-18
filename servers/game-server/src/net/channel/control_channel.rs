use super::ChannelImpl;

use std::borrow::Cow;

use bitstream_io::BitRead;
use fadia_engine::net::Bunch;
use fadia_engine::util::InBitReader;
use fadia_engine::util::{ReadPrimitivesExt, serialization::BitSerialize};

#[derive(Default)]
pub struct ControlChannel {
    pub received_messages: Vec<ControlChannelMessage>,
}

impl ChannelImpl for ControlChannel {
    fn received_bunch(
        &mut self,
        _: &mut super::World,
        bunch: &Bunch,
        r: &mut InBitReader,
    ) -> std::io::Result<()> {
        if bunch.bunch_data_bits != 0 {
            self.received_messages
                .push(ControlChannelMessage::decode(r)?);
        }

        Ok(())
    }
}

macro_rules! define_control_channel_messages {
    ($($name:ident, $index:expr $(, $pname:ident: $ptype:ty)*);*) => {
        $(#[allow(unused)]
        pub struct $name($(pub $ptype, )*);
        ::paste::paste!(pub const [<NMT_ $name:upper>]: u8 = $index;);)*

        $(#[allow(unused)]
        impl $name {
            pub fn decode<R: BitRead>(r: &mut R) -> ::std::io::Result<Self> {
                Ok(Self($(discard_and_do!($ptype, BitSerialize::read_from_bits(r)?), )*))
            }

            pub fn send(connection: &mut crate::net::connection::NetConnection, $($pname: $ptype, )*) -> ::std::io::Result<()> {
                use ::std::io::Cursor;
                use ::bitstream_io::{BitWriter, BitWrite, LittleEndian};

                let mut buf = Vec::with_capacity(128);
                let mut w = BitWriter::endian(Cursor::new(&mut buf), LittleEndian);

                ::paste::paste!(w.write(8, [<NMT_ $name:upper>])?;);
                $($pname.write_to_bits(&mut w)?;)*
                w.write_bit(true)?; // termination
                w.byte_align()?;
                connection.send_control_channel_reliable_bunch(&buf)
            }
        })*

        #[allow(unused)]
        pub enum ControlChannelMessage {
            Unknown(u8),
            $($name($name)),*
        }

        impl ControlChannelMessage {
            pub fn decode<R: BitRead>(r: &mut R) -> ::std::io::Result<Self> {
                ::paste::paste!(Ok(match r.read_u8()? {
                    $([<NMT_ $name:upper>] => Self::$name($name::decode(r)?),)*
                    unknown => Self::Unknown(unknown),
                }))
            }
        }
    };
}

macro_rules! discard_and_do {
    ($discarded:ty, $expr:expr) => {
        $expr
    };
}

define_control_channel_messages! {
    Hello, 0, is_little_endian: u8, network_version: u32, encryption_token: String, network_features: u16;
    Welcome, 1, map: Cow<'static, str>, game_name: Cow<'static, str>, redirect_url: Cow<'static, str>;
    Challenge, 3, challenge: String;
    Netspeed, 4, net_speed: u32;
    Login, 5, client_response: String, request_url: String, flags: u8, unique_id: String, online_platform: String;
    Join, 9
}
