//! Register definitions
use bitfield::bitfield;

// Register fields for command.regno (16-bit)
// CSR: 0x0000 - 0x0fff
pub const MARCHID: u16 = 0xF12;
pub const MIMPID: u16 = 0xF13;
pub const MSTATUS: u16 = 0x300;
pub const MISA: u16 = 0x301;
pub const MTVEC: u16 = 0x305;
pub const MSCRATCH: u16 = 0x340;
pub const MEPC: u16 = 0x341;
pub const MCAUSE: u16 = 0x342;
pub const MTVAL: u16 = 0x343;
pub const DPC: u16 = 0x7b1;

pub const DMDATA0: u8 = 0x04;
pub const DMDATA1: u8 = 0x05;
pub const DMCONTROL: u8 = 0x10;
pub const DMSTATUS: u8 = 0x11;
pub const DMHARTINFO: u8 = 0x12;
pub const DMABSTRACTCS: u8 = 0x16;
pub const DMCOMMAND: u8 = 0x17;
pub const DMABSTRACTAUTO: u8 = 0x18;
pub const DMPROGBUF0: u8 = 0x20;
pub const DMPROGBUF1: u8 = 0x21;
pub const DMPROGBUF2: u8 = 0x22;
pub const DMPROGBUF3: u8 = 0x23;
pub const DMPROGBUF4: u8 = 0x24;
pub const DMPROGBUF5: u8 = 0x25;
pub const DMPROGBUF6: u8 = 0x26;
pub const DMPROGBUF7: u8 = 0x27;

// GPR: 0x1000 - 0x101f
pub const GPRS: [(&str, &str, u16); 32] = [
    ("x0", "zero", 0x1000),
    ("x1", "ra", 0x1001),
    ("x2", "sp", 0x1002),
    ("x3", "gp", 0x1003),
    ("x4", "tp", 0x1004),
    ("x5", "t0", 0x1005),
    ("x6", "t1", 0x1006),
    ("x7", "t2", 0x1007),
    ("x8", "s0", 0x1008),
    ("x9", "s1", 0x1009),
    ("x10", "a0", 0x100a),
    ("x11", "a1", 0x100b),
    ("x12", "a2", 0x100c),
    ("x13", "a3", 0x100d),
    ("x14", "a4", 0x100e),
    ("x15", "a5", 0x100f),
    ("x16", "a6", 0x1010),
    ("x17", "a7", 0x1011),
    ("x18", "s2", 0x1012),
    ("x19", "s3", 0x1013),
    ("x20", "s4", 0x1014),
    ("x21", "s5", 0x1015),
    ("x22", "s6", 0x1016),
    ("x23", "s7", 0x1017),
    ("x24", "s8", 0x1018),
    ("x25", "s9", 0x1019),
    ("x26", "s10", 0x101a),
    ("x27", "s11", 0x101b),
    ("x28", "t3", 0x101c),
    ("x29", "t4", 0x101d),
    ("x30", "t5", 0x101e),
    ("x31", "t6", 0x101f),
];

/// Gereral Purpose Register for riscv32ec
pub const GPRS_RV32EC: [(&str, &str, u16); 16] = [
    ("x0", "zero", 0x1000),
    ("x1", "ra", 0x1001),
    ("x2", "sp", 0x1002),
    ("x3", "gp", 0x1003),
    ("x4", "tp", 0x1004),
    ("x5", "t0", 0x1005),
    ("x6", "t1", 0x1006),
    ("x7", "t2", 0x1007),
    ("x8", "s0", 0x1008),
    ("x9", "s1", 0x1009),
    ("x10", "a0", 0x100a),
    ("x11", "a1", 0x100b),
    ("x12", "a2", 0x100c),
    ("x13", "a3", 0x100d),
    ("x14", "a4", 0x100e),
    ("x15", "a5", 0x100f),
];

pub const CSRS: [(&str, u16); 16] = [
    ("marchid", 0xf12),
    ("mimpid", 0xf13),
    ("mhartid", 0xf14),
    ("misa", 0x301),
    ("mtvec", 0x305),
    ("mscratch", 0x340),
    ("mepc", 0x341),
    ("mcause", 0x342),
    ("mtval", 0x343),
    ("mstatus", 0x300),
    ("dcsr", 0x7b0),
    // ("dpc", 0x7b1),
    ("dscratch0", 0x7b2),
    ("dscratch1", 0x7b3),
    ("gintenr", 0x800),
    ("intsyscr", 0x804),
    ("corecfgr", 0xbc0),
];

// FPR: 0x1020-0x103f

/// Debug Module Register
pub trait DMReg: From<u32> + Into<u32> {
    const ADDR: u8;
}

bitfield! {
    /// Debug Module Control, 0x10
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
    /// Debug Module Status, 0x11
    pub struct Dmstatus(u32);
    impl Debug;
    pub allhavereset, _: 19;
    pub anyhavereset, _: 18;
    pub allresumeack, _: 17;
    pub anyresumeack, _: 16;
    pub allunavail, _: 13; // ? allavail
    pub anyunavail, _: 12;
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

bitfield! {
    /// Hart information register, Microprocessor status, 0x12
    pub struct Hartinfo(u32);
    impl Debug;
    pub nscratch, _: 23, 20;
    pub dataaccess, _: 16;
    pub datasize, _: 15, 12;
    pub dataaddr, _: 11, 0;
}
impl From<u32> for Hartinfo {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
impl From<Hartinfo> for u32 {
    fn from(val: Hartinfo) -> Self {
        val.0
    }
}
impl DMReg for Hartinfo {
    const ADDR: u8 = 0x12;
}

bitfield! {
    /// Abstract command status register, 0x16
    #[derive(Clone, Copy)]
    pub struct Abstractcs(u32);
    impl Debug;
    pub progbufsize, _: 28, 24;
    pub busy, _: 12;
    pub cmderr, set_cmderr: 10, 8;
    pub datacount, _: 3, 0;
}
impl From<u32> for Abstractcs {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
impl From<Abstractcs> for u32 {
    fn from(val: Abstractcs) -> Self {
        val.0
    }
}
impl DMReg for Abstractcs {
    const ADDR: u8 = 0x16;
}

bitfield! {
    /// Abstract command register
    pub struct Command(u32);
    impl Debug;
    pub cmdtype, set_cmdtype: 31, 24;
    pub aarsize, set_aarsize: 22, 20;
    pub aarpostincrement, set_aarpostincrement: 19;
    pub postexec, set_postexec: 18;
    pub transfer, set_transfer: 17;
    pub write, set_write: 16;
    pub regno, set_regno: 15, 0;
}
impl From<u32> for Command {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
impl From<Command> for u32 {
    fn from(val: Command) -> Self {
        val.0
    }
}
impl DMReg for Command {
    const ADDR: u8 = 0x17;
}
