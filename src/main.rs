use anyhow::Result;
use wlink::{
    commands::{self, DmiOp},
    device::WchLink,
    transport::Transport,
};

fn main() -> Result<()> {
    env_logger::init();
    let mut link = WchLink::open_nth(0)?;

    let resp = link.send_command(commands::control::GetProbeInfo)?;
    println!("probe info: {resp:?}");

    let r = link.send_command(commands::control::AttachChip)?;
    println!("chip info: {r:?}");

    let protected = link.send_command(commands::GetFlashProtected)?;
    println!("protected => {protected:?}");

    let uid = link.send_command(commands::GetChipId)?;
    println!("UID => {uid}");

    // read csr
    let marchid = link.read_csr(0xf12)?;
    // marchid => dc68d882
    // Parsed marchid: WCH-V4B
    println!("marchid => {:08x}", marchid);
    println!(
        "Parsed marchid: {}{}{}-{}{}{}",
        (((marchid >> 26) & 0x1F) + 64) as u8 as char,
        (((marchid >> 21) & 0x1F) + 64) as u8 as char,
        (((marchid >> 16) & 0x1F) + 64) as u8 as char,
        (((marchid >> 10) & 0x1F) + 64) as u8 as char,
        ((((marchid >> 5) & 0x1F) as u8) + b'0') as char,
        ((marchid & 0x1F) + 64) as u8 as char,
    );

    // link.resume_mcu()?;
    let mem = link.read_memory(0x08000000, 0x1000)?;
    println!("=> {:02x?}", mem);

    //link.send_command(commands::Reset::Quit)?;

    //let r = link.send_command(commands::control::DetachChip)?;
    //println!("detach => {:?}", r);

    Ok(())
}
