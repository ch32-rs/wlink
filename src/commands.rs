// pub mod dmi;

use crate::error::Result;

pub mod control;
pub trait Command {
    type Response: Response;
    const COMMAND_ID: u8;
    fn payload(&self) -> Vec<u8>;
    fn to_raw(&self) -> Vec<u8> {
        let mut bytes = vec![0x81, Self::COMMAND_ID, 0x00];
        bytes.extend(self.payload());
        bytes[2] = bytes.len() as u8 - 3;
        bytes
    }
}

pub trait Response {
    fn from_bytes(bytes: &[u8]) -> Result<Self>
    where
        Self: Sized;
}

impl Response for () {
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(())
    }
}

impl Response for Vec<u8> {
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(bytes.to_vec())
    }
}

pub struct GetChipProtected;
impl Command for GetChipProtected {
    type Response = bool;
    const COMMAND_ID: u8 = 0x06;
    fn payload(&self) -> Vec<u8> {
        vec![0x01]
    }
}
impl Response for bool {
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 1 {
            return Err(crate::error::Error::InvalidPayloadLength);
        }
        if bytes[0] == 0x01 {
            Ok(true)
        } else if bytes[0] == 0x02 {
            Ok(false)
        } else {
            Err(crate::error::Error::InvalidPayload)
        }
    }
}

/// Does not use standard response.
/// ffff00 20 aeb4abcd 16c6bc45 e339e339e339e339
///   cd-ab-b4-ae-45-bc-c6-16
pub struct GetChipId;
impl Command for GetChipId {
    type Response = Vec<u8>;
    const COMMAND_ID: u8 = 0x11;
    fn payload(&self) -> Vec<u8> {
        vec![0x09]
    }
}

pub enum Reset {
    Quit,
}
impl Command for Reset {
    type Response = ();
    const COMMAND_ID: u8 = 0x0b;
    fn payload(&self) -> Vec<u8> {
        match self {
            Reset::Quit => vec![0x01],
            // TODO: 0x02, 0x03
        }
    }
}

pub enum DmiOp {
    Nop,
    Read { addr: u8 },
    Write { addr: u8, data: u32 },
}
impl DmiOp {
    pub fn nop() -> Self {
        Self::Nop
    }
    pub fn read(addr: u8) -> Self {
        Self::Read { addr }
    }
    pub fn write(addr: u8, data: u32) -> Self {
        Self::Write { addr, data }
    }
}
impl Command for DmiOp {
    type Response = DmiOpResponse;
    const COMMAND_ID: u8 = 0x08;
    fn payload(&self) -> Vec<u8> {
        const DMI_OP_READ: u8 = 1;
        const DMI_OP_WRITE: u8 = 2;
        let mut bytes = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        match self {
            DmiOp::Nop => (),
            DmiOp::Read { addr } => {
                bytes[0] = *addr;
                bytes[5] = DMI_OP_READ;
            }
            DmiOp::Write { addr, data } => {
                bytes[0] = *addr;
                bytes[5] = DMI_OP_WRITE;
                bytes[1..5].copy_from_slice(&data.to_be_bytes());
            }
        }
        bytes
    }
}
#[derive(Debug)]
pub struct DmiOpResponse {
    pub addr: u8,
    pub data: u32,
    pub op: u8,
}
impl Response for DmiOpResponse {
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 6 {
            return Err(crate::error::Error::InvalidPayloadLength);
        }
        let addr = bytes[0];
        let op = bytes[5];
        let data = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
        Ok(DmiOpResponse { addr, data, op })
    }
}
