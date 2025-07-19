use std::io::{Cursor, Write};

use byteorder::{ByteOrder, WriteBytesExt};
use num_enum::{IntoPrimitive, TryFromPrimitive};

pub struct Packet {
    buf: Box<[u8]>,
    header_len: usize,
    payload_len: usize,
}

impl From<Packet> for Box<[u8]> {
    fn from(value: Packet) -> Self {
        value.buf
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
pub enum MessageID {
    GatewayClientLoginReq = 1106,
    GatewayClientLoginRes = 1107,
    GatewayClientTravelCmd = 1117,
    GatewayServerVersionCmd = 1120,
    GatewayClientKeepAliveCmd = 1121,
    GatewayServerKeepAliveCmd = 1122,
    GameDataFirstSync = 1652,
}

impl Packet {
    pub fn new<T: Into<Box<[u8]>>>(buf: T) -> Self {
        let buf = buf.into();
        let header_len = byteorder::LE::read_u32(&buf) as usize;
        let payload_len = byteorder::LE::read_u32(&buf[8 + header_len..]) as usize;

        Self {
            buf,
            header_len,
            payload_len,
        }
    }

    pub fn build(message_id: MessageID, header: &[u8], payload: &[u8]) -> Self {
        use byteorder::LE;

        let mut buf = Vec::with_capacity(12 + header.len() + payload.len());
        let mut w = Cursor::new(&mut buf);

        w.write_u32::<LE>(header.len() as u32).unwrap();
        w.write_all(header).unwrap();
        w.write_u32::<LE>(message_id.into()).unwrap();
        w.write_u32::<LE>(payload.len() as u32).unwrap();
        w.write_all(payload).unwrap();

        Self {
            buf: buf.into_boxed_slice(),
            header_len: header.len(),
            payload_len: payload.len(),
        }
    }

    pub fn message_id(&self) -> Result<MessageID, u32> {
        let message_id = byteorder::LE::read_u32(&self.buf[4 + self.header_len..]);
        message_id.try_into().map_err(|_| message_id)
    }

    pub fn header(&self) -> &[u8] {
        &self.buf[4..self.header_len + 4]
    }

    pub fn payload(&self) -> &[u8] {
        &self.buf[12 + self.header_len..12 + self.header_len + self.payload_len]
    }
}
