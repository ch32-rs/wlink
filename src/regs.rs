//! Register definitions
use bitfield::bitfield;

pub const MARCHID: u16 = 0xF12;
pub const MIMPID: u16 = 0xF13;
pub const MSTATUS: u16 = 0x300;
pub const MISA: u16 = 0x301;

/// Debug Module Register
pub trait DMReg: From<u32> + Into<u32> {
    const ADDR: u8;
}

bitfield! {
    /// Debug Module Control
    pub struct Dmcontrol(u32);
    impl Debug;
    pub haltreq, set_haltreq: 31;
    pub resumereq, set_resumereq: 30;
    pub ackhavereset, set_ackhavereset: 29;
    pub ndmreset, set_ndmreset: 1;
    pub dmactive, set_dmactive: 0;
}

impl From<u32> for Dmcontrol {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<Dmcontrol> for u32 {
    fn from(val: Dmcontrol) -> Self {
        val.0
    }
}

impl DMReg for Dmcontrol {
    const ADDR: u8 = 0x10;
}

bitfield! {
    /// Debug Module Status
    pub struct Dmstatus(u32);
    impl Debug;
    pub allhavereset, _: 19;
    pub anyhavereset, _: 18;
    pub allresumeack, _: 17;
    pub anyresumeack, _: 16;
    pub allavail, _: 13;
    pub anyavail, _: 12;
    pub allrunning, _: 11;
    pub anyrunning, _: 10;
    pub allhalted, _: 9;
    pub anyhalted, _: 8;
    pub authenticated, _: 7;
    pub version, _: 3, 0;
}

impl From<u32> for Dmstatus {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<Dmstatus> for u32 {
    fn from(val: Dmstatus) -> Self {
        val.0
    }
}

impl DMReg for Dmstatus {
    const ADDR: u8 = 0x11;
}
