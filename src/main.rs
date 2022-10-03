use std::{
    path::{Path, PathBuf},
    sync::{mpsc::RecvTimeoutError, Arc, Mutex},
    time::Duration,
};

use anyhow::{bail, Result};
use clap::Parser;
use ppk2::{
    measurement::{Measurement, MeasurementMatch},
    types::{DevicePower, Level as PinLevel, LogicPortPins, MeasurementMode, SourceVoltage},
    Ppk2,
};
use probe_rs::{
    flashing::{DownloadOptions, Format},
    Permissions, Probe, Session,
};
use tracing::{error, info, trace, warn, Level as LogLevel};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
struct Args {
    #[arg()]
    elf: PathBuf,

    #[arg(
        short,
        long,
        help = "The number of tests the device will run. If omitted, the ELF will be inspected to infer the number of tests."
    )]
    num_tests: Option<usize>,

    #[arg(short, long, help = "The chip to program")]
    chip: String,

    #[arg(
        short = 'p',
        long,
        help = "The serial port the PPK2 is connected to. If unspecified, will try to find the PPK2 automatically"
    )]
    serial_port: Option<String>,

    #[arg(
        short = 'v',
        long,
        help = "The voltage of the device source in mV",
        default_value = "0"
    )]
    voltage: SourceVoltage,

    #[arg(short = 'm', long, help = "Measurement mode", default_value = "source")]
    mode: MeasurementMode,

    #[arg(short = 'l', long, help = "The log level", default_value = "info")]
    log_level: LogLevel,

    #[arg(
        short = 's',
        long,
        help = "The maximum number of samples to be taken per second. Uses averaging of device samples Samples are analyzed in chunks, and as such the actual number of samples per second will deviate. Note that a lower value may result in signal pin transition going unnoticed.",
        default_value = "1000"
    )]
    sps: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(args.log_level)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let expected_test_count = match args.num_tests {
        Some(n) => n,
        None => read_test_count(&args.elf)?,
    };

    let ppk2_port = match args.serial_port {
        Some(p) => p,
        None => ppk2::try_find_ppk2_port()?,
    };

    let mut ppk2 = Ppk2::new(ppk2_port, args.mode)?;
    ppk2.set_source_voltage(args.voltage)?;

    // Power on
    ppk2.set_device_power(DevicePower::Enabled)?;

    // Flash firmware
    let mut session = attach_probe(&args.chip)?;
    flash_firmware(&mut session, &args.elf)?;

    // Halt core
    let mut core = session.core(0)?;
    core.reset_and_halt(Duration::from_secs(60))?;

    // TODO power off
    // TODO disconnect debugger somehow
    // TODO power on

    // Start measuring, ignoring data if D0 has not been high yet, or if it is high
    let mut levels = [PinLevel::Either; 8];
    levels[0] = PinLevel::Low;
    let pins = LogicPortPins::with_levels(levels);
    let (rx, kill) = ppk2.start_measurement_matching(pins, args.sps)?;

    // Setup signal handler, stopping measurements on SIGKILL
    let kill = Arc::new(Mutex::new(Some(kill)));
    let kill_in_handler = kill.clone();
    ctrlc::set_handler(move || {
        let mut ppk2 = kill_in_handler.lock().unwrap().take().unwrap()().unwrap();
        ppk2.set_device_power(DevicePower::Disabled).unwrap();
        std::process::exit(0);
    })?;

    // Whether a preamble has been detected this run. The preamble
    // is a state where the port state does not match, that is, D0 is high.
    // This state must be detected before reporting starts, in order for the device
    // to get ready for testing.
    let mut preamble_detected = false;
    // The current reports cumulative current
    let mut sum = 0f32;
    // The number of measurements done in this report, used to calculate the average
    let mut count = 0;
    // The number of reports that have finished this run.
    let mut report_count = 0;
    // Reset core in order to start tests
    core.reset()?;

    let ppk2 = loop {
        let rcv_res = rx.recv_timeout(Duration::from_millis(2000));
        if report_count >= expected_test_count {
            // The expected number of tests have ran and have been reported.
            break kill.lock().unwrap().take().map(|k| k()).unwrap();
        }
        use MeasurementMatch::*;
        match rcv_res {
            // Measurement digital port state matched, add data to current report
            Ok(Match(Measurement { micro_amps, pins })) if preamble_detected => {
                count += 1;
                sum += micro_amps;
                trace!("Last: {:.4} mA. Bits: {:?}", micro_amps / 1000., pins);
            }
            // Digital port state does not match requirements, so either:
            // - No test has started yet. We mark the preample having been detected,
            //   so the next match is detected as a test being run, and data collection will start
            // - The last test has ended, and we report its average current use. The next time the
            //   port state matches, a new report is set up.
            Ok(NoMatch) => {
                preamble_detected = true;
                if count > 0 {
                    // 7. Report average current use for each test measurement
                    report_count += 1;
                    info!(
                        "Average current for report {report_count}: {:.8} mA",
                        (sum / count as f32) / 1000.
                    )
                }
                count = 0;
                sum = 0.;
                trace!("No match, ignoring.");
            }
            // We got a match, but no preamble yet.
            Ok(m) => {
                trace!("No preamble detected yet {m:?}");
            }
            // The sender was closed, so we run the kill function.
            Err(RecvTimeoutError::Disconnected) => {
                break kill.lock().unwrap().take().map(|k| k()).unwrap()
            }
            // Something else bad happended. Report and break.
            Err(e) => {
                error!("Error receiving data: {e:?}");
                break Err(e)?;
            }
        }
    };
    if let Ok(mut ppk2) = ppk2 {
        // Power off
        ppk2.set_device_power(DevicePower::Disabled)?;
    }
    info!("Goodbye!");
    Ok(())
}

/// Read the number of tests the device will run from the ELF.
/// This function assumes [defmt-test] is used to set up the test binary,
/// as it uses the `DEFMT_TEST_COUNT` symbol value exposed in the ELF.
/// This function fails if it cannot find this symbol.
fn read_test_count(elf_file_path: impl AsRef<Path>) -> Result<usize> {
    use object::{Object, ObjectSection, ObjectSymbol};

    let bin_data = std::fs::read(elf_file_path)?;
    let elf = object::File::parse(&*bin_data)?;

    let symbol = match elf.symbols().find(|s| s.name() == Ok("DEFMT_TEST_COUNT")) {
        Some(s) => s,
        None => bail!("symbol DEFMT_TEST_COUNT not found"),
    };

    let section = elf.section_by_index(symbol.section().index().unwrap())?;
    let data = section
        .data_range(symbol.address(), symbol.size())?
        .unwrap();

    // Account for word size and endianness
    let count = match (elf.is_little_endian(), elf.is_64()) {
        (true, false) => u32::from_le_bytes(data.try_into().unwrap()) as usize,
        (false, false) => u32::from_be_bytes(data.try_into().unwrap()) as usize,
        (true, true) => u64::from_le_bytes(data.try_into().unwrap()) as usize,
        (false, true) => u64::from_be_bytes(data.try_into().unwrap()) as usize,
    };

    Ok(count)
}

/// Try to attach a probe given the chip name.
fn attach_probe(chip: &str) -> Result<Session> {
    info!("Attaching to probe for chip {chip}");
    let mut probes = Probe::list_all().into_iter();
    let session = loop {
        let probe = match probes.next() {
            Some(p) => p,
            None => bail!("No probe found for chip {chip}"),
        };
        let probe = match probe.open() {
            Ok(p) => p,
            Err(e) => {
                warn!("Could not open probe: {e}");
                continue;
            }
        };
        match probe.attach(chip, Permissions::new().allow_erase_all()) {
            Ok(s) => break s,
            Err(_) => continue,
        };
    };
    Ok(session)
}

/// Try to flash the firmware from the ELF file to the connected chip.
fn flash_firmware(session: &mut Session, elf: impl AsRef<Path>) -> Result<()> {
    let elf = elf.as_ref();
    info!("Start flashing {}...", elf.to_string_lossy());
    let mut options = DownloadOptions::default();
    options.verify = true;
    options.do_chip_erase = true;
    probe_rs::flashing::download_file_with_options(session, elf, Format::Elf, options)?;
    info!("Done!");
    Ok(())
}
