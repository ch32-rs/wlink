//! WCH-Link commands and response types.

use std::fmt;
use std::fmt::Debug;

use crate::error::{Error, Result};

// 0x0d subset
pub mod control;

/// Command to call the WCH-Link
pub trait Command: Debug {
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
            if reason == 0x55 {
                return Err(Error::Protocol(reason, resp.to_vec()));
            }
            Err(Error::Protocol(reason, resp.to_vec()))
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

/// Generic raw command
#[derive(Debug)]
pub struct RawCommand<const N: u8>(pub Vec<u8>);
impl<const N: u8> Command for RawCommand<N> {
    type Response = Vec<u8>;
    const COMMAND_ID: u8 = N;
    fn payload(&self) -> Vec<u8> {
        self.0.clone()
    }
}

/// Set address and offset of the firmware, 0x01.
#[derive(Debug)]
pub struct SetWriteMemoryRegion {
    // 0x08000000 or 0x00000000
    pub start_addr: u32,
    pub len: u32,
}
impl Command for SetWriteMemoryRegion {
    type Response = ();
    const COMMAND_ID: u8 = 0x01;
    fn payload(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8);
        bytes.extend_from_slice(&self.start_addr.to_be_bytes());
        bytes.extend_from_slice(&self.len.to_be_bytes());
        bytes
    }
}

/// Read a block of memory from the chip.
#[derive(Debug)]
pub struct SetReadMemoryRegion {
    pub start_addr: u32,
    pub len: u32,
}
impl Command for SetReadMemoryRegion {
    type Response = ();
    const COMMAND_ID: u8 = 0x03;
    fn payload(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8);
        bytes.extend_from_slice(&self.start_addr.to_be_bytes());
        bytes.extend_from_slice(&self.len.to_be_bytes());
        bytes
    }
}

/// 0x02 subset
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Program {
    // wlink_erase
    EraseFlash = 0x01,
    // Before write firmware bytes, choice between 0x02 and 0x04
    WriteFlash = 0x02,
    // after write flash
    WriteFlashAndVerify = 0x04,
    /// Write Flash OP
    WriteFlashOP = 0x05,
    // before SetRamAddress
    Prepare = 0x06,
    /// Unknown, maybe commit flash op written
    Unknown07AfterFlashOPWritten = 0x07, // or 0x0B for riscvchip=1
    /// Unknown, maybe commit flash op written, only for riscvchip=1
    Unknown0BAfterFlashOPWritten = 0x0B,
    // EndProgram
    End = 0x08,
    /// Read memory section
    ReadMemory = 0x0c,
}
impl Command for Program {
    type Response = u8;
    const COMMAND_ID: u8 = 0x02;
    fn payload(&self) -> Vec<u8> {
        vec![*self as u8]
    }
}

/// 0x06 subset
// query -> check -> set
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum FlashProtect {
    /// 06, _, 01
    CheckReadProtect, // 1 for protected, 2 for unprotected
    /// 06, _, 02
    Unprotect,
    /// 06, _, 03
    Protect,
    /// 06, _, 04
    CheckReadProtectEx, // 1 for protected, 0 for unprotected,
    /// bf, or e7
    UnprotectEx(u8),    // with 0xbf, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    // prefix byte 0xe7 ? for ch32x035
    ProtectEx(u8), // with 0xbf, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
}
impl FlashProtect {
    pub const FLAG_PROTECTED: u8 = 0x01;
}
impl Command for FlashProtect {
    type Response = u8;
    const COMMAND_ID: u8 = 0x06;
    fn payload(&self) -> Vec<u8> {
        use FlashProtect::*;
        match *self {
            CheckReadProtect => vec![0x01],
            Unprotect => vec![0x02],
            Protect => vec![0x03],
            CheckReadProtectEx => vec![0x04],
            UnprotectEx(b) => vec![0x02, b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
            ProtectEx(b) => vec![0x03, b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        }
    }
}

#[derive(Debug)]
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
pub enum GetChipInfo {
    V1 = 0x09,
    // spot on WCH-LinkUtility v1.70
    V2 = 0x06,
}
impl Command for GetChipInfo {
    type Response = ChipUID;
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
pub struct ChipUID(pub [u8; 8]);
impl Response for ChipUID {
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
impl fmt::Display for ChipUID {
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
impl fmt::Debug for ChipUID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("ChipId({:02x?})", &self.0[..]))
    }
}

/// Device reset (0x0b, _)
#[derive(Debug)]
pub enum Reset {
    /// wlink_quitreset
    ResetAndRun, // the most common reset
    Normal,
    Normal2,
}
impl Command for Reset {
    type Response = ();
    const COMMAND_ID: u8 = 0x0b;
    fn payload(&self) -> Vec<u8> {
        match self {
            Reset::ResetAndRun => vec![0x01],
            Reset::Normal => vec![0x03],
            Reset::Normal2 => vec![0x02],
        }
    }
}

/// Speed settings
#[derive(Debug, Copy, Clone, clap::ValueEnum, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Speed {
    /// 400
    Low = 0x03,
    /// 4000
    Medium = 0x02,
    /// 6000
    High = 0x01,
}
impl Default for Speed {
    fn default() -> Self {
        Speed::High
    }
}

/// Set CLK Speed, 0x0C
#[derive(Debug)]
pub struct SetSpeed {
    pub riscvchip: u8,
    pub speed: Speed,
}
impl Command for SetSpeed {
    type Response = bool;
    const COMMAND_ID: u8 = 0x0c;
    fn payload(&self) -> Vec<u8> {
        vec![self.riscvchip, self.speed as u8]
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
#[derive(Debug)]
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

#[derive(Debug)]
pub struct DisableDebug;
impl Command for DisableDebug {
    type Response = ();
    const COMMAND_ID: u8 = 0x0e;
    // 0x81, 0x0e, 0x01, 0x01
    fn payload(&self) -> Vec<u8> {
        vec![0x01]
    }
}

// 81 0D 05 11 SetAccessAddress
// 81 0F 01 02 GetDeviceMode
// 81 0D 01 07 EnableQE
// 81 0D 01 06 CheckQE
// 81 FE 01 00 DisEncrypt
// 81 0D 01 0F ClearCodeFlashB
// 81 0D 02 08 xx ClearCodeFlash
// 81 11 01 0D unkown in query info, before GetChipRomRamSplit

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chip_id_parsing() {
        let raw = hex::decode("ffff0020aeb4abcd16c6bc45e339e339e339e339").unwrap();

        let uid = ChipUID::from_raw(&raw).unwrap();

        println!("=> {uid:?}");
        assert_eq!("cd-ab-b4-ae-45-bc-c6-16", uid.to_string());
    }
}
