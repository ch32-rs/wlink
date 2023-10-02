use std::{thread::sleep, time::Duration};

use anyhow::Result;
use wlink::{
    commands::{self, RawCommand},
    device::WchLink,
    dmi::DebugModuleInterface,
    format::read_firmware_from_file,
    regs, RiscvChip,
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

    /// Detach chip after operation
    #[arg(long, global = true, default_value = "true")]
    detach: bool,

    /// Specify the chip type, e.g. CH32V30X
    #[arg(long, global = true)]
    chip: Option<RiscvChip>,

    /// Connection Speed
    #[arg(long, global = true, default_value = "high")]
    speed: crate::commands::Speed,

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
    /// Program the code flash
    Flash {
        /// Address in u32
        #[arg(short, long, value_parser = parse_number)]
        address: Option<u32>,
        /// Do not erase flash before flashing
        #[arg(long, short = 'E', default_value = "false")]
        no_erase: bool,
        /// Do not reset and run after flashing
        #[arg(long, short = 'R', default_value = "false")]
        no_run: bool,
        /// Path to the firmware file to flash
        path: String,
    },
    /// Unlock flash
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
    Dev {},
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
            probe.set_speed(cli.speed);
            probe.probe_info()?;
            probe.attach_chip(cli.chip)?;
            match command {
                Dev {} => {
                    const FLASH_KEYR: u32 = 0x2000_0030;
                    let mut algo = wlink::dmi::Algorigthm::new(&mut probe);
                    // algo.write_mem32(FLASH_KEYR, 0x45670123)?;

                    //algo.ensure_mcu_halt()?;
                    let address = 0x40001041;
                    let v = algo.read_mem32(address)?;
                    println!("0x{:08x}: 0x{:08x}", address, v);

                    // algo.dump_pmp()?;
                }
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

                    let dmstatus: regs::Dmstatus = probe.read_dmi_reg()?;
                    log::info!("{dmstatus:#?}");
                }
                Erase {} => {
                    log::info!("Erase Flash...");
                    probe.erase_flash()?;
                    log::debug!("Wait for some time to finish erase...");
                    sleep(Duration::from_millis(1000));
                }
                Flash {
                    address,
                    no_erase,
                    no_run,
                    path,
                } => {
                    let firmware = read_firmware_from_file(path)?;
                    let start_address =
                        address.unwrap_or_else(|| probe.chip.as_ref().unwrap().memory_start_addr);
                    log::info!(
                        "Flashing {} bytes to 0x{:08x}",
                        firmware.len(),
                        start_address
                    );

                    if !no_erase {
                        log::info!("Erase Flash");
                        probe.erase_flash()?;
                    }

                    probe.write_flash(&firmware, start_address)?;
                    log::info!("Flash done");

                    sleep(Duration::from_millis(500));

                    if !no_run {
                        log::info!("Now reset...");
                        probe.send_command(commands::Reset::AndRun)?;
                        sleep(Duration::from_millis(500));
                    }
                    // reattach
                    //probe.attach_chip(cli.chip)?;
                    //log::info!("Resume executing...");
                    //probe.ensure_mcu_resume()?;
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
                    let dmstatus: regs::Dmstatus = probe.read_dmi_reg()?;
                    log::info!("{dmstatus:#x?}");
                    let dmcontrol: regs::Dmcontrol = probe.read_dmi_reg()?;
                    log::info!("{dmcontrol:#x?}");
                    let hartinfo: regs::Hartinfo = probe.read_dmi_reg()?;
                    log::info!("{hartinfo:#x?}");
                    let abstractcs: regs::Abstractcs = probe.read_dmi_reg()?;
                    log::info!("{abstractcs:#x?}");
                    let haltsum0 = probe.dmi_read(0x40)?;
                    log::info!("haltsum0: {:#x?}", haltsum0);

                    let cpbr = probe.dmi_read(0x7E)?;
                    log::info!("cpbr: {:#x?}", cpbr);
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
    let s = s.replace("_", "").to_lowercase();
    if s.starts_with("0x") {
        Ok(u32::from_str_radix(&s[2..], 16)
            .unwrap_or_else(|_| panic!("error while parsering {s:?}")))
    } else if s.starts_with("0b") {
        Ok(u32::from_str_radix(&s[2..], 2)
            .unwrap_or_else(|_| panic!("error while parsering {s:?}")))
    } else {
        Ok(s.parse().expect("must be a number"))
    }
}
