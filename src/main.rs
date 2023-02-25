use anyhow::Result;
use wlink::{
    commands,
    device::WchLink,
    regs::{Dmcontrol, Dmstatus},
};

fn main() -> Result<()> {
    env_logger::init();
    let mut probe = WchLink::open_nth(0)?;

    probe.attach_chip()?;

    /*loop {
        link.resume_mcu()?;
        std::thread::sleep(std::time::Duration::from_millis(10000));
        link.halt_mcu()?;
        std::thread::sleep(std::time::Duration::from_millis(10000));
    }*/

    /*

    let dmcontrol: Dmcontrol = probe.dmi_read()?;
    println!("=> {:?}", dmcontrol);

    //println!("resume");
    // probe.resume_mcu()?;
    let dmstatus: Dmstatus = probe.dmi_read()?;
    println!("=> {:?}", dmstatus);

    let dmcontrol: Dmcontrol = probe.dmi_read()?;
    println!("=> {:?}", dmcontrol);
    */

    //let firmware = include_bytes!("../firmware.bin");
    //println!("flash {} bytes", firmware.len());
    //link.write_flash(firmware)?;

    //let dmstatus: Dmstatus = probe.dmi_read()?;
    //println!("=> {:?}", dmstatus);

    probe.halt_mcu()?;
    // probe.resume_mcu()?;

    //probe.reset_mcu_and_run()?;

    // probe.read_flash_size_kb()?;

    let _mem = probe.read_memory(0x0800_0000, 0x200)?;
    // println!("{}", nu_pretty_hex::pretty_hex(&mem));

    //link.reset_mcu_and_run()?;
    //log::info!("reset dm");
    //probe.reset_debug_module()?;

    //link.send_command(commands::Reset::Quit)?;

    // link.send_command(commands::control::DetachChip)?;

    Ok(())
}
