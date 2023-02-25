use std::{thread::sleep, time::Duration};

use anyhow::Result;
use wlink::{
    commands,
    device::WchLink,
    regs::{Dmcontrol, Dmstatus},
};

use clap::{Parser, Subcommand};

#[derive(clap::Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Optional device index to operate on
    #[arg(long, short = 'd', value_name = "INDEX")]
    device: Option<usize>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Dump memory region
    Dump {
        /// Start address
        #[arg(value_parser = parse_number)]
        address: u32,

        /// Length in bytes, will be rounded up to the next multiple of 4
        #[arg(value_parser = parse_number)]
        length: u32,
    },
    /// Program the flash
    Flash {
        /// Path to the binary file to flash
        path: String,
    },
    /// Halts the MCU
    Halt {},
    /// Resumes the MCU
    Resume {},
    /// Reset the MCU
    Reset {},
}

fn main() -> Result<()> {
    use Commands::*;

    let cli = Cli::parse();
    // init simplelogger

    if cli.verbose >= 2 {
        let _ = simplelog::TermLogger::init(
            simplelog::LevelFilter::Trace,
            simplelog::Config::default(),
            simplelog::TerminalMode::Mixed,
            simplelog::ColorChoice::Auto,
        );
    } else if cli.verbose == 1 {
        let _ = simplelog::TermLogger::init(
            simplelog::LevelFilter::Debug,
            simplelog::Config::default(),
            simplelog::TerminalMode::Mixed,
            simplelog::ColorChoice::Auto,
        );
    } else {
        let _ = simplelog::TermLogger::init(
            simplelog::LevelFilter::Info,
            simplelog::Config::default(),
            simplelog::TerminalMode::Mixed,
            simplelog::ColorChoice::Auto,
        );
    }

    let device_index = cli.device.unwrap_or(0);

    let mut probe = WchLink::open_nth(device_index)?;
    probe.probe_info()?;

    match cli.command {
        None => {
            println!("No command given, doing nothing");
        }
        Some(command) => {
            probe.attach_chip()?;
            match command {
                Dump { address, length } => {
                    log::info!(
                        "Read memory from 0x{:08x} to 0x{:08x}",
                        address,
                        address + length
                    );
                    probe.read_memory(address, length)?;
                }
                Halt {} => {
                    log::info!("Halt MCU");
                    probe.halt_mcu()?;
                }
                Resume {} => {
                    log::info!("Resume MCU");
                    probe.resume_mcu()?;
                }
                Flash { path } => {
                    let firmware = std::fs::read(path)?;
                    log::info!("flash {} bytes", firmware.len());
                    probe.write_flash(&firmware)?;
                    log::info!("flash done");
                }
                Reset {} => {
                    probe.send_command(commands::Reset::Quit)?;
                    log::info!("reset");
                    sleep(Duration::from_millis(300));
                    probe.resume_mcu()?;
                }
                _ => todo!(),
            }
        }
        _ => todo!(),
    }

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

    // probe.resume_mcu()?;

    //probe.reset_mcu_and_run()?;

    // probe.read_flash_size_kb()?;

    // println!("{}", nu_pretty_hex::pretty_hex(&mem));

    //link.reset_mcu_and_run()?;
    //log::info!("reset dm");
    //probe.reset_debug_module()?;

    //link.send_command(commands::Reset::Quit)?;

    // link.send_command(commands::control::DetachChip)?;

    Ok(())
}

pub fn parse_number(s: &str) -> std::result::Result<u32, String> {
    if s.starts_with("0x") || s.starts_with("0X") {
        Ok(u32::from_str_radix(&s[2..], 16).expect(&format!("error while parsering {:?}", s)))
    } else if s.starts_with("0b") || s.starts_with("0B") {
        Ok(u32::from_str_radix(&s[2..], 2).expect(&format!("error while parsering {:?}", s)))
    } else {
        Ok(s.parse().expect("must be a number"))
    }
}
