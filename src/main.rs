use std::fs::File;
use std::io::prelude::*;
use std::time::Instant;
use clap::{Arg, App, AppSettings, SubCommand};
use clap::{value_t, crate_description, crate_version};
use spi_flash::Flash;

use jtagdap::probe::{Probe, ProbeInfo};
use jtagdap::dap::DAP;
use spidap::SPIFlash;

#[allow(clippy::cognitive_complexity)]
fn main() -> anyhow::Result<()> {
    let matches = App::new("spidap")
        .version(crate_version!())
        .about(crate_description!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .global_setting(AppSettings::ColoredHelp)
        .global_setting(AppSettings::DeriveDisplayOrder)
        .global_setting(AppSettings::GlobalVersion)
        .global_setting(AppSettings::InferSubcommands)
        .global_setting(AppSettings::VersionlessSubcommands)
        .arg(Arg::with_name("quiet")
             .help("Suppress informative output")
             .long("quiet")
             .short("q")
             .global(true))
        .arg(Arg::with_name("probe")
             .help("VID:PID[:SN] of CMSIS-DAP device to use")
             .long("probe")
             .short("p")
             .takes_value(true)
             .global(true))
        .arg(Arg::with_name("freq")
             .help("JTAG clock frequency to use, in kHz")
             .long("freq")
             .short("f")
             .takes_value(true)
             .default_value("1000")
             .global(true))
        .arg(Arg::with_name("hold-reset")
            .help("Hold nRST asserted during operation")
            .long("hold-reset")
            .short("r")
            .global(true))
        .subcommand(SubCommand::with_name("probes")
            .about("List available CMSIS-DAP probes"))
        .subcommand(SubCommand::with_name("id")
            .about("Read SPI flash ID"))
        .subcommand(SubCommand::with_name("scan")
            .about("Read SPI flash parameters"))
        .subcommand(SubCommand::with_name("erase")
            .about("Erase entire SPI flash"))
        .subcommand(SubCommand::with_name("write")
            .about("Write binary file to SPI flash")
            .arg(Arg::with_name("file")
                 .help("File to write to SPI flash")
                 .required(true))
            .arg(Arg::with_name("offset")
                 .help("Start address (in bytes) to write to")
                 .long("offset")
                 .default_value("0"))
            .arg(Arg::with_name("no-verify")
                 .help("Disable readback verification")
                 .short("n")
                 .long("no-verify")))
        .subcommand(SubCommand::with_name("read")
            .about("Read SPI flash contents to file")
            .arg(Arg::with_name("file")
                 .help("File to write SPI flash contents to")
                 .required(true))
            .arg(Arg::with_name("offset")
                 .help("Start address (in bytes) of read")
                 .long("offset")
                 .takes_value(true)
                 .default_value("0"))
            .arg(Arg::with_name("length")
                 .help("Length (in bytes) of read, defaults to detected capacity")
                 .long("length")
                 .takes_value(true)))
        .subcommand(SubCommand::with_name("protect")
            .about("Set all block protection bits in status register"))
        .subcommand(SubCommand::with_name("unprotect")
            .about("Clear all block protection bits in status register"))
        .get_matches();

    pretty_env_logger::init_timed();
    let t0 = Instant::now();
    let quiet = matches.is_present("quiet");

    // Listing probes does not require first connecting to a probe,
    // so we just list them and quit early.
    if matches.subcommand_name().unwrap() == "probes" {
        print_probe_list();
        return Ok(());
    }

    // All functions after this point require an open probe, so
    // we now attempt to connect to the specified probe.
    let probe = if matches.is_present("probe") {
        ProbeInfo::from_specifier(matches.value_of("probe").unwrap())?.open()?
    } else {
        Probe::new()?
    };

    // Create a JTAG interface using the probe.
    let dap = DAP::new(probe)?;

    // If the user specified a JTAG clock frequency, apply it now.
    match value_t!(matches, "freq", u32) {
        Ok(freq) => dap.set_clock(freq * 1000)?,
        Err(e) => {
            drop(dap);
            e.exit();
        }
    }

    // If hold-reset is specified, assert nRST now.
    if matches.is_present("hold-reset") {
        dap.set_nrst(false)?;
    }

    // Create a Flash instance.
    let mut spi = SPIFlash::new(dap);
    let mut flash = Flash::new(&mut spi);

    // Always bring flash out of power-down, as
    // for example iCE40s love to put it into
    // power-down after booting.
    flash.release_power_down()?;

    // Always read parameter table if available, to load
    // settings for address bytes, capacity, opcodes, etc.
    flash.read_params()?;

    match matches.subcommand_name() {
        Some("id") => {
            let id = flash.read_id()?;
            println!("{}", id);
        },
        Some("scan") => {
            if !quiet { println!("Reading flash ID...") };
            let id = flash.read_id()?;
            println!("{}", id);
            if !quiet { println!("\nReading flash parameters...") };
            match flash.get_params() {
                Some(params) => println!("{}", params),
                None => println!("No SFDP header found. Check flash supports SFDP."),
            }
            if !quiet { println!("Reading status registers...") };
            let status1 = flash.read_status1()?;
            let status2 = flash.read_status2()?;
            let status3 = flash.read_status3()?;
            println!("Status 1: 0x{:02X}, status 2: 0x{:02X}, status 3: 0x{:02X}",
                     status1.0, status2.0, status3.0);
            let (bp0, bp1, bp2) = status1.get_block_protect();
            let sec = status1.get_sec();
            let tb = status1.get_tb();
            println!("BP0: {}, BP1: {}, BP2: {}, SEC: {}, TB: {}", bp0, bp1, bp2, sec, tb);
        },
        Some("erase") => {
            if quiet {
                flash.erase()?;
            } else {
                flash.erase_progress()?;
            }
        },
        Some("write") => {
            if !quiet && flash.is_protected()? {
                println!("Flash appears to be write-protected; writing may fail.");
            }
            let matches = matches.subcommand_matches("write").unwrap();
            let path = matches.value_of("file").unwrap();
            let offset = value_t!(matches, "offset", u32).unwrap();
            let verify = !matches.is_present("no-verify");
            let mut file = File::open(path)?;
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;
            if quiet {
                flash.program(offset, &data, verify)?;
            } else {
                flash.program_progress(offset, &data, verify)?;
            }
        },
        Some("read") => {
            let matches = matches.subcommand_matches("read").unwrap();
            let path = matches.value_of("file").unwrap();
            let offset = value_t!(matches, "offset", u32).unwrap();
            let length = if matches.is_present("length") {
                value_t!(matches, "length", usize).unwrap()
            } else {
                log::info!("No length specified, autodetecting");
                if let Some(capacity) = flash.capacity() {
                    capacity
                } else {
                    println!("Could not detect flash capacity; specify --length instead.");
                    return Ok(());
                }
            };
            let mut file = File::create(path)?;
            let data = if quiet {
                flash.read(offset, length)?
            } else {
                flash.read_progress(offset, length)?
            };
            file.write_all(&data)?;
        },
        Some("protect") => {
            if !quiet { println!("Setting block protection bits...") };
            flash.protect(true, true, true)?;
            if !quiet { println!("All block protection bits set.") };
        },
        Some("unprotect") => {
            if !quiet { println!("Disabling flash write protection...") };
            flash.unprotect()?;
            if !quiet { println!("Flash protected disabled.") };
        },
        _ => panic!("Unhandled command."),
    }

    // If hold-reset is specified, de-assert nRST now.
    if matches.is_present("hold-reset") {
        spi.release().set_nrst(true)?;
    }

    let t1 = t0.elapsed();
    if !quiet {
        println!("Finished in {}.{:02}s", t1.as_secs(), t1.subsec_millis()/10);
    }

    Ok(())
}

fn print_probe_list() {
    let probes = ProbeInfo::list();
    if probes.is_empty() {
        println!("No CMSIS-DAP probes found.");
    } else {
        println!("Found {} CMSIS-DAP probe{}:", probes.len(),
                 if probes.len() == 1 { "" } else { "s" });
        for probe in probes {
            println!("  {}", probe);
        }
    }
}
