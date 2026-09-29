//! Probe control commands. COMMAND_ID = 0x0d

use crate::{RiscvChip, probe::WchLinkVariant};

use super::*;

/// GetDeviceVersion (0x0d, 0x01)
#[derive(Debug)]
pub struct GetProbeInfo;
impl Command for GetProbeInfo {
    type Response = ProbeInfo;
    const COMMAND_ID: u8 = 0x0d;
    fn payload(&self) -> Vec<u8> {
        vec![0x01]
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProbeInfo {
    pub major_version: u8,
    pub minor_version: u8,
    pub variant: WchLinkVariant,
}
impl ProbeInfo {
    /// The raw `major.minor` bytes as reported by the probe.
    ///
    /// Prefer [`ProbeInfo::version_code`] for comparisons and
    /// [`ProbeInfo::display_version`] for showing the version to a human: the
    /// raw bytes are not the human-readable version (see `version_code`).
    pub fn version(&self) -> (u8, u8) {
        (self.major_version, self.minor_version)
    }

    /// The two version bytes packed into a single value: `16 * major + minor`.
    pub fn packed_version(&self) -> u16 {
        16 * self.major_version as u16 + self.minor_version as u16
    }

    /// The monotonic firmware version code, used for comparisons.
    ///
    /// The probe reports a packed version, not a decimal one. Below `0x30` the
    /// code is the decimal reading, at or above it the packed value is shifted
    /// by 12:
    ///
    /// ```text
    /// V = if packed < 0x30 { 10 * major + minor } else { packed - 12 }
    /// ```
    ///
    /// This makes the code increase by one per release across the `0x30`
    /// boundary: `v2.15` is 35, `v3.0` (= raw `2.16`) is 36, `v3.6`
    /// (= raw `2.22`) is 42.
    pub fn version_code(&self) -> u16 {
        Self::version_code_of(self.major_version, self.minor_version)
    }

    /// The version code for an arbitrary `major.minor` pair, see
    /// [`ProbeInfo::version_code`].
    pub const fn version_code_of(major: u8, minor: u8) -> u16 {
        let packed = 16 * major as u16 + minor as u16;
        if packed < 0x30 {
            10 * major as u16 + minor as u16
        } else {
            packed - 12
        }
    }

    /// The human-readable `(major, minor)`, decoded from the packed value.
    ///
    /// The two parts are the nibbles of the packed version, so excess minor
    /// values carry into the major part: raw `2.22` is really `3.6`.
    pub fn display_version(&self) -> (u16, u16) {
        let packed = self.packed_version();
        (packed >> 4, packed & 0xF)
    }
}

/// First WCH-Link firmware known to report the CH32V205 family correctly,
/// i.e. v3.6 (older builds spell it `2.22`); both map to version code 42.
pub const MIN_FW_VERSION_CH32V205: u16 = ProbeInfo::version_code_of(3, 6);

/// `GetChipInfo::V2` was introduced in firmware v2.9, version code 29.
pub const MIN_FW_VERSION_CHIP_INFO_V2: u16 = ProbeInfo::version_code_of(2, 9);
impl Response for ProbeInfo {
    fn from_payload(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 3 {
            return Err(crate::error::Error::InvalidPayloadLength);
        }
        Ok(Self {
            major_version: bytes[0],
            minor_version: bytes[1],
            // Only avaliable in newer version of firmware
            variant: if bytes.len() == 4 {
                WchLinkVariant::try_from_u8(bytes[2])?
            } else {
                WchLinkVariant::Ch549
            },
        })
    }
}
impl fmt::Display for ProbeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (major, minor) = self.display_version();
        write!(
            f,
            "WCH-Link v{}.{}(v{}) ({})",
            major,
            minor,
            self.version_code(),
            self.variant
        )
    }
}

/// ?SetChipType (0x0d, 0x02)
#[derive(Debug)]
pub struct AttachChip;
impl Command for AttachChip {
    type Response = AttachChipResponse;
    const COMMAND_ID: u8 = 0x0d;
    fn payload(&self) -> Vec<u8> {
        vec![0x02]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttachChipResponse {
    pub chip_family: RiscvChip,
    pub riscvchip: u8,
    pub chip_id: u32,
}
impl Response for AttachChipResponse {
    fn from_payload(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 5 {
            return Err(Error::InvalidPayloadLength);
        }
        Ok(Self {
            chip_family: RiscvChip::try_from_u8(bytes[0])?,
            riscvchip: bytes[0],
            chip_id: u32::from_be_bytes(bytes[1..5].try_into().unwrap()),
        })
    }
}
// For logging
impl fmt::Display for AttachChipResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.chip_id == 0 {
            write!(f, "{:?}", self.chip_family)
        } else if let Some(chip_name) = crate::chips::chip_id_to_chip_name(self.chip_id) {
            write!(
                f,
                "{:?} [{}] (ChipID: 0x{:08x})",
                self.chip_family, chip_name, self.chip_id
            )
        } else {
            write!(f, "{:?} (ChipID: 0x{:08x})", self.chip_family, self.chip_id)
        }
    }
}

/// Erase code flash, only supported by WCH-LinkE.
#[derive(Debug)]
pub enum EraseCodeFlash {
    ByPinRST(RiscvChip),
    ByPowerOff(RiscvChip),
}
impl Command for EraseCodeFlash {
    type Response = ();
    const COMMAND_ID: u8 = 0x0d;

    fn payload(&self) -> Vec<u8> {
        match self {
            // This is more complex, require RST pin to be connected.
            EraseCodeFlash::ByPinRST(c) => vec![0x08, *c as u8],
            // NOTE: From the protocol, this command's bytes is wrongly seted
            // 81 0d 01 0f 09, note here, the "length" bytes is wrong.
            // I guess it is not verified. So here we use `02`.
            EraseCodeFlash::ByPowerOff(c) => vec![0x0f, *c as u8],
        }
    }
}

/// GetROMRAM, Only avaliable for CH32V2, CH32V3, CH56X
/// 0, 1, 2, 3
#[derive(Debug)]
pub struct GetChipRomRamSplit;
impl Command for GetChipRomRamSplit {
    type Response = u8;
    const COMMAND_ID: u8 = 0x0d;
    fn payload(&self) -> Vec<u8> {
        vec![0x04]
    }
}

/// 0, 1, 2, 3
#[derive(Debug)]
pub struct SetChipRomRamSplit(u8);
impl Command for SetChipRomRamSplit {
    type Response = ();
    const COMMAND_ID: u8 = 0x0d;
    fn payload(&self) -> Vec<u8> {
        vec![0x05, self.0]
    }
}

// ?? close out
/// Detach Chip, (0x0d, 0xff)
#[derive(Debug)]
pub struct OptEnd;
impl Command for OptEnd {
    type Response = ();
    const COMMAND_ID: u8 = 0x0d;
    fn payload(&self) -> Vec<u8> {
        vec![0xff]
    }
}

/// Set Power, from pow3v3, pow5v fn
#[derive(clap::Subcommand, PartialEq, Clone, Copy, Debug)]
pub enum SetPower {
    /// Enable 3.3V output
    Enable3v3,
    /// Disable 3.3V output
    Disable3v3,
    /// Enable 5V output
    Enable5v,
    /// Disable 5V output
    Disable5v,
}
impl Command for SetPower {
    type Response = ();
    const COMMAND_ID: u8 = 0x0d;
    fn payload(&self) -> Vec<u8> {
        match self {
            SetPower::Enable3v3 => vec![0x09],
            SetPower::Disable3v3 => vec![0x0A],
            SetPower::Enable5v => vec![0x0B],
            SetPower::Disable5v => vec![0x0C],
        }
    }
}

/// SDI print support, only available for WCH-LinkE
/// Firmware version >= 2.10
#[derive(Debug)]
pub struct SetSdiPrintEnabled(pub bool);

impl Command for SetSdiPrintEnabled {
    // 0x00 success, 0xff not support
    type Response = u8;
    const COMMAND_ID: u8 = 0x0d;
    fn payload(&self) -> Vec<u8> {
        if self.0 {
            vec![0xee, 0x00]
        } else {
            vec![0xee, 0x01]
        }
    }
}

/// Set RST pin
#[derive(Debug)]
pub enum SetRSTPin {
    Low,
    High,
    Floating,
}
impl Command for SetRSTPin {
    type Response = ();
    const COMMAND_ID: u8 = 0x0d;
    fn payload(&self) -> Vec<u8> {
        let subcmd = match *self {
            SetRSTPin::Low => 0x13,
            SetRSTPin::High => 0x14,
            SetRSTPin::Floating => 0x15,
        };
        vec![0x0e, subcmd]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(major: u8, minor: u8) -> ProbeInfo {
        ProbeInfo {
            major_version: major,
            minor_version: minor,
            variant: WchLinkVariant::ECh32v305,
        }
    }

    #[test]
    fn version_code_matches_wch_numbering() {
        // Below 0x30 the code is the decimal reading.
        assert_eq!(ProbeInfo::version_code_of(2, 8), 28);
        assert_eq!(ProbeInfo::version_code_of(2, 9), 29);
        assert_eq!(ProbeInfo::version_code_of(2, 10), 30);
        assert_eq!(ProbeInfo::version_code_of(2, 11), 31);
        assert_eq!(ProbeInfo::version_code_of(2, 15), 35);
        // At/above 0x30 the packed value is shifted by 12.
        assert_eq!(ProbeInfo::version_code_of(2, 16), 36);
        assert_eq!(ProbeInfo::version_code_of(2, 22), 42);
        assert_eq!(ProbeInfo::version_code_of(3, 6), 42);
    }

    #[test]
    fn version_code_is_monotonic_across_the_0x30_boundary() {
        // Within the 2.x/3.x range the code increases by exactly one per packed
        // step, including across the 0x30 boundary (v2.15 -> v3.0).
        let mut prev: Option<u16> = None;
        for packed in 0x20..0x40u16 {
            let code = ProbeInfo::version_code_of((packed / 16) as u8, (packed % 16) as u8);
            if let Some(prev) = prev {
                assert_eq!(
                    code,
                    prev + 1,
                    "version code jumped at packed {packed:#04x}"
                );
            }
            prev = Some(code);
        }
        // The boundary itself: v2.15 is 35, v3.0 (raw 2.16) is 36.
        assert_eq!(ProbeInfo::version_code_of(2, 15), 35);
        assert_eq!(ProbeInfo::version_code_of(2, 16), 36);
    }

    #[test]
    fn display_version_unpacks_the_nibbles() {
        // The two user-visible cases: the reported minor is not the real minor.
        assert_eq!(info(2, 22).display_version(), (3, 6));
        assert_eq!(info(2, 10).display_version(), (2, 10));
        assert_eq!(info(2, 8).display_version(), (2, 8));
        assert_eq!(info(2, 16).display_version(), (3, 0));
    }

    #[test]
    fn display_formats_normalized_version_and_code() {
        // Regression: 2.22(v42) was wrong, it is 3.6(v42).
        assert_eq!(
            info(2, 22).to_string(),
            "WCH-Link v3.6(v42) (WCH-LinkE-CH32V305)"
        );
        assert_eq!(
            info(2, 10).to_string(),
            "WCH-Link v2.10(v30) (WCH-LinkE-CH32V305)"
        );
        assert_eq!(
            info(2, 8).to_string(),
            "WCH-Link v2.8(v28) (WCH-LinkE-CH32V305)"
        );
    }

    #[test]
    fn ch32v205_gate_accepts_3_6_and_rejects_older() {
        assert!(info(2, 22).version_code() >= MIN_FW_VERSION_CH32V205);
        assert!(info(3, 6).version_code() >= MIN_FW_VERSION_CH32V205);
        assert!(info(2, 21).version_code() < MIN_FW_VERSION_CH32V205);
    }
}
