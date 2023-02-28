use std::{thread::sleep, time::Duration};

use anyhow::Result;
use wlink::{commands, device::WchLink, regs};

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

    /// Detach chip after operation
    #[arg(long, global = true)]
    detach: bool,

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
    /// Dump registers
    Regs {},
    /// Erase flash
    Erase {},
    /// Program the flash
    Flash {
        /// Path to the binary file to flash
        path: String,
    },
    /// Force set register
    WriteReg {
        /// Reg in u16
        #[arg(value_parser = parse_number)]
        reg: u32,
        /// Value in u32
        #[arg(value_parser = parse_number)]
        value: u32,
    },
    /// Force write a memory word
    WriteMem {
        /// Address in u32
        #[arg(value_parser = parse_number)]
        address: u32,
        /// Value in u32
        #[arg(value_parser = parse_number)]
        value: u32,
    },
    /// Halts the MCU
    Halt {},
    /// Resumes the MCU
    Resume {},
    /// Reset the MCU
    Reset {},
    /// Debug, check status
    Status {},
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
            println!("No command given, use --help for help.");
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
                Regs {} => {
                    log::info!("Dump GPRs");
                    probe.dump_regs()?;
                }
                Halt {} => {
                    log::info!("Halt MCU");
                    probe.ensure_mcu_halt()?;
                }
                Resume {} => {
                    log::info!("Resume MCU");
                    probe.ensure_mcu_resume()?;
                }
                Erase {} => {
                    log::info!("Erase flash");
                    probe.erase_flash()?;
                }
                Flash { path } => {
                    let firmware = std::fs::read(path)?;
                    log::info!("flash {} bytes", firmware.len());
                    probe.write_flash(&firmware)?;
                    log::info!("flash done");

                    sleep(Duration::from_secs(1));

                    log::info!("now reset...");
                    probe.send_command(commands::Reset::Quit)?;
                    sleep(Duration::from_secs(1));
                    log::info!("resume executing...");
                    probe.ensure_mcu_resume()?;
                }
                Reset {} => {
                    // probe.send_command(commands::Reset::Quit)?;
                    probe.soft_reset()?;
                    log::info!("soft reset");
                    sleep(Duration::from_millis(300));
                    probe.ensure_mcu_resume()?;
                }
                WriteReg { reg, value } => {
                    let regno = reg as u16;
                    log::info!("set reg 0x{:04x} to 0x{:08x}", regno, value);
                    probe.write_reg(regno, value)?;
                }
                WriteMem { address, value } => {
                    log::info!("write memory 0x{:08x} to 0x{:08x}", value, address);
                    probe.write_memory_word(address, value)?;
                }
                Status {} => {
                    let dmstatus: regs::Dmstatus = probe.dmi_read()?;
                    println!("=> {dmstatus:?}");
                }
            }
        }
    }

    if cli.detach {
        probe.detach_chip()?;
    }

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
        Ok(u32::from_str_radix(&s[2..], 16)
            .unwrap_or_else(|_| panic!("error while parsering {s:?}")))
    } else if s.starts_with("0b") || s.starts_with("0B") {
        Ok(u32::from_str_radix(&s[2..], 2)
            .unwrap_or_else(|_| panic!("error while parsering {s:?}")))
    } else {
        Ok(s.parse().expect("must be a number"))
    }
}
