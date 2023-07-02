//! WCH-Link commands and response types.

use std::fmt;

use crate::error::{Error, Result};

// 0x0d subset
pub mod control;

/// Command to call the WCH-Link
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

/// Response type of a command call
pub trait Response {
    /// parse the PAYLOAD part only
    fn from_payload(bytes: &[u8]) -> Result<Self>
    where
        Self: Sized;
    /// default implementation for parsing [0x82 CMD LEN PAYLOAD] style response
    fn from_raw(resp: &[u8]) -> Result<Self>
    where
        Self: Sized,
    {
        if resp[0] == 0x81 {
            let reason = resp[1];
            let len = resp[2] as usize;
            if len != resp[3..].len() {
                return Err(Error::InvalidPayloadLength);
            }
            let payload = resp[3..3 + len].to_vec();
            Err(Error::Protocol(reason, payload))
        } else if resp[0] == 0x82 {
            let len = resp[2] as usize;
            if len != resp[3..].len() {
                return Err(Error::InvalidPayloadLength);
            }
            let payload = resp[3..3 + len].to_vec();
            Self::from_payload(&payload)
        } else {
            Err(Error::InvalidPayload)
        }
    }
}

impl Response for () {
    fn from_payload(_bytes: &[u8]) -> Result<Self> {
        Ok(())
    }
}

impl Response for Vec<u8> {
    fn from_payload(bytes: &[u8]) -> Result<Self> {
        Ok(bytes.to_vec())
    }
}

impl Response for u8 {
    fn from_payload(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 1 {
            return Err(Error::InvalidPayloadLength);
        }
        Ok(bytes[0])
    }
}

pub struct RawCommand<const N: u8>(pub Vec<u8>);
impl<const N: u8> Command for RawCommand<N> {
    type Response = Vec<u8>;
    const COMMAND_ID: u8 = N;
    fn payload(&self) -> Vec<u8> {
        self.0.clone()
    }
}

/// Set RAM address (0x08000000)
pub struct SetRamAddress {
    // 0x08000000 or 0x00000000
    pub start_addr: u32,
    pub len: u32,
}
impl Command for SetRamAddress {
    type Response = ();
    const COMMAND_ID: u8 = 0x01;
    fn payload(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8);
        bytes.extend_from_slice(&self.start_addr.to_be_bytes());
        bytes.extend_from_slice(&self.len.to_be_bytes());
        bytes
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Program {
    // wlink_erase
    EraseFlash = 0x01,
    BeginWriteFlash = 0x02,
    // after write memory
    BeginWriteMemory = 0x05,

    ExecMemory = 0x07,
    // EndProgram
    End = 0x08,
    // after read memory
    BeginReadMemory = 0x0c,
}
impl Command for Program {
    type Response = u8;
    const COMMAND_ID: u8 = 0x02;
    fn payload(&self) -> Vec<u8> {
        vec![*self as u8]
    }
}

/// Read a block of memory from the chip.
pub struct ReadMemory {
    pub start_addr: u32,
    pub len: u32,
}
impl Command for ReadMemory {
    type Response = ();
    const COMMAND_ID: u8 = 0x03;
    fn payload(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8);
        bytes.extend_from_slice(&self.start_addr.to_be_bytes());
        bytes.extend_from_slice(&self.len.to_be_bytes());
        bytes
    }
}

/// (0x06, _)
// query -> check -> set
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum FlashProtect {
    Query = 0x01,   // 1 for protected, 2 for unprotected
    QueryV2 = 0x04, // 1 for protected, 0 for unprotected,
    Protect = 0x03,
    // dummy 0xf0 mask
    ProtectV2 = 0xf3, // with 0xbf, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    Unprotect = 0x02,
    // dummy 0xf0 mask
    UnprotectV2 = 0xf2, // with 0xbf, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
}
impl FlashProtect {
    pub const FLAG_PROTECTED: u8 = 0x01;
}
impl Command for FlashProtect {
    type Response = u8;
    const COMMAND_ID: u8 = 0x06;
    fn payload(&self) -> Vec<u8> {
        match *self {
            FlashProtect::ProtectV2 => vec![0x03, 0xbf, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
            FlashProtect::UnprotectV2 => vec![0x02, 0xbf, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
            _ => vec![*self as u8],
        }
    }
}

pub struct SetFlashProtected {
    pub protected: bool,
}
impl Command for SetFlashProtected {
    type Response = u8;
    const COMMAND_ID: u8 = 0x06;
    fn payload(&self) -> Vec<u8> {
        if self.protected {
            vec![0x03]
        } else {
            vec![0x02]
        }
    }
}

/// Get Chip UID, the UID is also avaliable in the `wchisp` command.
// ??? 0x11, 0x01, _ (riscvchip)
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum QueryChipInfo {
    V1 = 0x09,
    // spot on WCH-LinkUtility v1.70
    V2 = 0x06,
}
impl Command for QueryChipInfo {
    type Response = ChipId;
    const COMMAND_ID: u8 = 0x11;
    fn payload(&self) -> Vec<u8> {
        vec![*self as u8]
    }
}

// This does not use standard response format:
// raw response: ffff00 20 aeb4abcd 16c6bc45 e339e339e339e339
// UID in wchisp: cd-ab-b4-ae-45-bc-c6-16
// e339e339e339e339 => inital value of erased flash
/// Chip UID, also reported by wchisp
pub struct ChipId(pub [u8; 8]);
impl Response for ChipId {
    fn from_raw(resp: &[u8]) -> Result<Self> {
        if resp.len() <= 12 {
            return Err(Error::InvalidPayloadLength);
        }
        if &resp[..2] == b"\xff\xff" {
            let mut bytes = [0u8; 8];
            bytes[0..4]
                .copy_from_slice(&u32::from_be_bytes(resp[4..8].try_into().unwrap()).to_le_bytes());
            bytes[4..8].copy_from_slice(
                &u32::from_be_bytes(resp[8..12].try_into().unwrap()).to_le_bytes(),
            );
            Ok(Self(bytes))
        } else {
            log::warn!("cannot read chip id");
            Ok(Self(Default::default()))
        }
    }

    fn from_payload(_bytes: &[u8]) -> Result<Self> {
        unreachable!("ChipId is not be parsed from payload; qed")
    }
}
impl fmt::Display for ChipId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            &self
                .0
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join("-"),
        )
    }
}
impl fmt::Debug for ChipId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("ChipId({:02x?})", &self.0[..]))
    }
}

/// Device reset (0x0b, _)
pub enum Reset {
    /// wlink_quitreset
    Quit,
    Normal,
    Normal2,
}
impl Command for Reset {
    type Response = ();
    const COMMAND_ID: u8 = 0x0b;
    fn payload(&self) -> Vec<u8> {
        match self {
            Reset::Quit => vec![0x01],
            Reset::Normal => vec![0x03],
            Reset::Normal2 => vec![0x02],
        }
    }
}

/// (0x0c, [riscvchip, 1/2/3])
pub struct SetTwoLineMode {
    pub riscvchip: u8,
    pub speed: u8, // 1, 2, 3
}

impl Command for SetTwoLineMode {
    type Response = bool;
    const COMMAND_ID: u8 = 0x0c;
    fn payload(&self) -> Vec<u8> {
        vec![self.riscvchip, self.speed]
    }
}

impl Response for bool {
    fn from_payload(resp: &[u8]) -> Result<Self> {
        if resp.len() != 1 {
            return Err(Error::InvalidPayloadLength);
        }
        Ok(resp[0] == 0x01) // 1 means success
    }
}

/// DMI operations
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
        const DMI_OP_NOP: u8 = 0;
        const DMI_OP_READ: u8 = 1;
        const DMI_OP_WRITE: u8 = 2;
        let mut bytes = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        match self {
            DmiOp::Nop => {
                bytes[5] = DMI_OP_NOP; // :)
            }
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

// DMI_STATUS_SUCCESS = 0,
// DMI_STATUS_FAILED = 2,
// DMI_STATUS_BUSY = 3
#[derive(Debug)]
pub struct DmiOpResponse {
    pub addr: u8,
    pub data: u32,
    pub op: u8,
}
impl DmiOpResponse {
    pub fn is_busy(&self) -> bool {
        self.op == 0x03
    }

    pub fn is_success(&self) -> bool {
        self.op == 0x00
    }

    // should read mcause to get the reason
    pub fn is_failed(&self) -> bool {
        self.op == 0x03 || self.op == 0x02
    }
}
impl Response for DmiOpResponse {
    fn from_payload(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 6 {
            return Err(Error::InvalidPayloadLength);
        }
        let addr = bytes[0];
        let op = bytes[5];
        let data = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
        Ok(DmiOpResponse { addr, data, op })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chip_id_parsing() {
        let raw = hex::decode("ffff0020aeb4abcd16c6bc45e339e339e339e339").unwrap();

        let uid = ChipId::from_raw(&raw).unwrap();

        println!("=> {uid:?}");
        assert_eq!("cd-ab-b4-ae-45-bc-c6-16", uid.to_string());
    }
}
