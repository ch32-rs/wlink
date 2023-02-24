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

    let uid = link.send_command(commands::GetChipId);
    println!("=> {}", uid);

    // read csr
    link.send_command(DmiOp::write(0x10, 0x80000001))?;
    link.send_command(DmiOp::write(0x10, 0x80000001))?;
    link.send_command(DmiOp::write(0x10, 0x00000001))?;
    link.send_command(DmiOp::write(0x04, 0x00000000))?;
    link.send_command(DmiOp::write(0x17, 0x00220f12))?; // 0xf12 marchid
    link.send_command(DmiOp::read(0x16))?;
    let marchid = link.send_command(DmiOp::read(0x04))?;
    println!("marchid => {:08x}", marchid.data);
    let marchid = marchid.data;
    println!(
        "{}{}{}-{}{}{}",
        (((marchid >> 26) & 0x1F) + 64) as u8 as char,
        (((marchid >> 21) & 0x1F) + 64) as u8 as char,
        (((marchid >> 16) & 0x1F) + 64) as u8 as char,
        (((marchid >> 10) & 0x1F) + 64) as u8 as char,
        ((((marchid >> 5) & 0x1F) as u8) + b'0') as char,
        ((marchid & 0x1F) + 64) as u8 as char,
    );

    // resume sequence
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
