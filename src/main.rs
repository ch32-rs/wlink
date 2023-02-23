use anyhow::Result;
use wlink::{
    commands::{self, DmiOp},
    transport::{Transport, WchLink},
};

fn main() -> Result<()> {
    env_logger::init();
    let mut link = WchLink::open_nth(0)?;

    let resp = link.send_command(commands::control::GetProbeInfo)?;

    println!("=> {:?}", resp);

    let r = link.send_command(commands::control::AttachChip)?;
    println!("=> {:?}", r);

    let protected = link.send_command(commands::GetChipProtected)?;
    println!("protected => {:?}", protected);

    let _ = link.send_command(commands::GetChipId);
    //    println!("=> {:02x?}", r);

    /// reset csr
    link.send_command(DmiOp::write(0x10, 0x80000001))?;
    link.send_command(DmiOp::write(0x10, 0x80000001))?;
    link.send_command(DmiOp::write(0x10, 0x00000001))?;
    link.send_command(DmiOp::write(0x04, 0x00000000))?;
    link.send_command(DmiOp::write(0x17, 0x00220f12))?; // 0xf12 marchid
    link.send_command(DmiOp::read(0x16))?;
    let marchid = link.send_command(DmiOp::read(0x04))?;
    println!("marchid => {:08x}", marchid.data);

    /// resume sequence
    link.send_command(DmiOp::write(0x10, 0x80000001))?;
    link.send_command(DmiOp::write(0x10, 0x80000001))?;
    link.send_command(DmiOp::write(0x10, 0x00000001))?;
    link.send_command(DmiOp::write(0x10, 0x40000001))?;
    link.send_command(DmiOp::read(0x11))?;

    //link.send_command(commands::Reset::Quit)?;

    //let r = link.send_command(commands::control::DetachChip)?;
    //println!("detach => {:?}", r);

    Ok(())
}
