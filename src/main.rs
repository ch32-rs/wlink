use std::{thread::sleep, time::Duration};

use anyhow::Result;
use wlink::{commands, device::WchLink, format::read_firmware_from_file, regs, RiscvChip};

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

    /// Specify the chip type, e.g. CH32V30X
    #[arg(long, global = true)]
    chip: Option<RiscvChip>,

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
        /// Address in u32
        #[arg(short, long, value_parser = parse_number)]
        address: u32,
        /// Path to the binary file to flash
        path: String,
    },
    /// Unlock flash, enable debugging
    Unprotect {},
    /// Protect flash
    Protect {},
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
    /// Swifth mode from RV to DAP or vice versa
    ModeSwitch {
        #[arg(long)]
        rv: bool,
        #[arg(long)]
        dap: bool,
    },
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

    match cli.command {
        None => {
            wlink::device::check_usb_device()?;
            println!("No command given, use --help for help.");
        }
        Some(ModeSwitch { rv, dap }) => {
            wlink::device::check_usb_device()?; // list all connected devices
            log::warn!("This is an experimental feature, better use the WCH-LinkUtility!");
            if !(rv ^ dap) {
                println!("Please choose one mode to switch, either --rv or --dap");
            } else if dap {
                wlink::device::try_switch_from_rv_to_dap(device_index)?;
            } else {
                wlink::device::try_switch_from_dap_to_rv(device_index)?;
            }
        }
        Some(command) => {
            let mut probe = WchLink::open_nth(device_index)?;
            probe.probe_info()?;
            probe.attach_chip(cli.chip)?;
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

                    let dmstatus: regs::Dmstatus = probe.dmi_read()?;
                    log::info!("{dmstatus:?}");
                }
                Erase {} => {
                    log::info!("Erase Flash");
                    probe.erase_flash()?;
                }
                Flash { address, path } => {
                    let firmware = read_firmware_from_file(path)?;

                    log::info!("Flashing {} bytes to 0x{:08x}", firmware.len(), address);
                    probe.write_flash(&firmware, address)?;
                    log::info!("Flash done");

                    sleep(Duration::from_millis(500));

                    log::info!("Now reset...");
                    probe.send_command(commands::Reset::Quit)?;
                    sleep(Duration::from_millis(500));
                    log::info!("Resume executing...");
                    probe.ensure_mcu_resume()?;
                }
                Unprotect {} => {
                    log::info!("Unprotect Flash");
                    probe.protect_flash(false)?;
                }
                Protect {} => {
                    log::info!("Protect Flash");
                    probe.protect_flash(true)?;
                }
                Reset {} => {
                    // probe.send_command(commands::Reset::Quit)?;
                    probe.soft_reset()?;
                    log::info!("Soft reset");
                    sleep(Duration::from_millis(300));
                    probe.ensure_mcu_resume()?;

                    // probe.reset_debug_module()?;
                }
                WriteReg { reg, value } => {
                    let regno = reg as u16;
                    log::info!("Set reg 0x{:04x} to 0x{:08x}", regno, value);
                    probe.write_reg(regno, value)?;
                }
                WriteMem { address, value } => {
                    log::info!("Write memory 0x{:08x} to 0x{:08x}", value, address);
                    probe.write_memory_word(address, value)?;
                }
                Status {} => {
                    probe.dump_info()?;
                    let dmstatus: regs::Dmstatus = probe.dmi_read()?;
                    log::info!("{dmstatus:#?}");
                }
                _ => unreachable!("unimplemented command"),
            }
            if cli.detach {
                probe.detach_chip()?;
            }
        }
    }

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
