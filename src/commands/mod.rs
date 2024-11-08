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
pub struct RawCommand<const N: u8>(pub Vec<u8>);
impl<const N: u8> Command for RawCommand<N> {
    type Response = Vec<u8>;
    const COMMAND_ID: u8 = N;
    fn payload(&self) -> Vec<u8> {
        self.0.clone()
    }
}
impl<const N: u8> fmt::Debug for RawCommand<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RawCommand<0x{:02x}>({})", N, hex::encode(&self.0))
    }
}

/// 0x01 - Set address and offset of the firmware
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

/// 0x02 - Flash or Memory operations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Program {
    // wlink_erase
    EraseFlash = 0x01,
    // Before write firmware bytes, choice between 0x02 and 0x04
    WriteFlash = 0x02,
    // Write flash
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
pub enum ConfigChip {
    /// 06, _, 01
    CheckReadProtect, // 1 for protected, 2 for unprotected
    /// 06, _, 02
    Unprotect,
    /// 06, _, 03
    Protect,
    /// 06, _, 04
    CheckReadProtectEx, // 1 for protected, 0 for unprotected,
    /// bf, or e7
    UnprotectEx(u8), // with 0xbf, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    // prefix byte 0xe7 ? for ch32x035
    ProtectEx(u8), // with 0xbf, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    /// Config flags
    /// 81 06 08 02 3f 00 00  ff ff ff ff
    /// __ __ __ ?? ?? [DATA] [WRP      ]
    Config {
        /// User data
        data: u16,
        /// WRP write protection
        wrp: u32,
    },
}
impl ConfigChip {
    pub const FLAG_READ_PROTECTED: u8 = 0x01;
    pub const FLAG_WRITE_PROTECTED: u8 = 0x11;
}
impl Command for ConfigChip {
    type Response = u8;
    const COMMAND_ID: u8 = 0x06;
    fn payload(&self) -> Vec<u8> {
        match *self {
            ConfigChip::CheckReadProtect => vec![0x01],
            ConfigChip::Unprotect => vec![0x02],
            ConfigChip::Protect => vec![0x03],
            // ret = 0x11 protected
            ConfigChip::CheckReadProtectEx => vec![0x04],
            // b = 0xff ?
            ConfigChip::UnprotectEx(b) => vec![0x02, b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
            // [0x03, 0xff, 0xff, 0xff, WPR0, WPR1, WPR2, WPR3]
            ConfigChip::ProtectEx(b) => vec![0x03, b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
            ConfigChip::Config { data: _, wrp: _ } => todo!("ConfigChip: config flags"),
        }
    }
}

/// Get Chip UID, the UID is also available in the `wchisp` command.
// ??? 0x11, 0x01, _ (riscvchip)
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum GetChipInfo {
    V1 = 0x09,
    // spot on WCH-LinkUtility v1.70
    V2 = 0x06,
}
impl Command for GetChipInfo {
    type Response = ESignature;
    const COMMAND_ID: u8 = 0x11;
    fn payload(&self) -> Vec<u8> {
        vec![*self as u8]
    }
}

// See-also: https://github.com/ch32-rs/wlink/issues/58
// This does not use standard response format:
// raw response: ffff0020 aeb4abcd 16c6bc45 e339e339 20360510
// UID in wchisp: cd-ab-b4-ae-45-bc-c6-16
// e339e339 => inital value of erased flash
// 20360510 => chip id
/// Flash size and Chip UID, also reported by wchisp
#[derive(Clone, PartialEq, Debug)]
pub struct ESignature {
    /// Non-zero-wait flash size in KB
    pub flash_size_kb: u16,
    /// UID
    pub uid: [u32; 2],
}

impl Response for ESignature {
    fn from_payload(_bytes: &[u8]) -> Result<Self>
    where
        Self: Sized,
    {
        unreachable!("ESignature is not be parsed from payload; qed")
    }

    fn from_raw(resp: &[u8]) -> Result<Self> {
        if resp.len() < 12 {
            return Err(Error::InvalidPayloadLength);
        }
        let flash_size_kb = u16::from_be_bytes(resp[2..4].try_into().unwrap());
        let uid = [
            u32::from_be_bytes(resp[4..8].try_into().unwrap()),
            u32::from_be_bytes(resp[8..12].try_into().unwrap()),
        ];
        Ok(Self { flash_size_kb, uid })
    }
}
impl fmt::Display for ESignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // write aa-bb-cc-.. style UID
        let bytes: [u8; 8] = unsafe { std::mem::transmute(self.uid) };
        write!(
            f,
            "FlashSize({}KB) UID({})",
            self.flash_size_kb,
            &bytes
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join("-")
        )
    }
}

/// Device reset (0x0b, _)
#[derive(Debug)]
pub enum Reset {
    /// wlink_quitreset, reset and run
    Soft, // the most common reset
    Normal,
    /// wlink_chip_reset, chip reset
    // The memory is not reset
    Chip,
}
impl Command for Reset {
    type Response = ();
    const COMMAND_ID: u8 = 0x0b;
    fn payload(&self) -> Vec<u8> {
        match self {
            Reset::Soft => vec![0x01],
            Reset::Normal => vec![0x03],
            Reset::Chip => vec![0x02],
        }
    }
}

/// Speed settings
#[derive(Debug, Copy, Clone, clap::ValueEnum, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub enum Speed {
    /// 400kHz
    Low = 0x03,
    /// 4000kHz
    Medium = 0x02,
    /// 6000kHz
    #[default]
    High = 0x01,
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
    pub fn read(addr: u8) -> Self {
        DmiOp::Read { addr }
    }
    pub fn write(addr: u8, data: u32) -> Self {
        DmiOp::Write { addr, data }
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
// 81 11 01 0D unknown in query info, before GetChipRomRamSplit
// 81 0D 02 EE 00/02/03 SetSDLineMode
// 81 0F 01 01 SetIAPMode
