use std::{
    sync::{mpsc::RecvTimeoutError, Arc, Mutex},
    time::Duration,
};

use anyhow::Result;
use ppk2::{
    measurement::{Measurement, MeasurementMatch},
    types::{DevicePower, Level as PinLevel, LogicPortPins, MeasurementMode, SourceVoltage},
    Ppk2,
};
use tracing::{error, info, Level, trace};
use tracing_subscriber::FmtSubscriber;
/*
Steps:
1. Power on
2. Flash device firmware
3. Power off
4. Disconnect debugger
5. Start measuring, ignoring data if D0 has not been high yet, or if it is high
6. Power on
7. Read test measurements i.e. data each time interval that D0 is low
8. power off
9. Report average current use for each test measurement

Steps 3, 4, and 6 can be omitted if it's not possible to disconnect the debugger
*/

const EXPECTED_TESTS: usize = 3;

fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let ppk2_port = ppk2::try_find_ppk2_port()?;
    let mut ppk2 = Ppk2::new(ppk2_port, MeasurementMode::Source)?;
    ppk2.set_source_voltage(SourceVoltage::from_millivolts(3300))?;
    // Steps:
    // 1. Power on
    ppk2.set_device_power(DevicePower::Enabled)?;
    // 2. Flash device firmware
    // 3. Power off
    // 4. Disconnect debugger
    // 5. Start measuring, ignoring data if D0 has not been high yet, or if it is high
    let mut levels = [PinLevel::Either; 8];
    levels[0] = PinLevel::Low;
    let pins = LogicPortPins::with_levels(levels);
    let (rx, kill) = ppk2.start_measurement_matching(pins, 100000)?;

    let kill = Arc::new(Mutex::new(Some(kill)));
    let kill_in_handler = kill.clone();
    ctrlc::set_handler(move || {
        let mut ppk2 = kill_in_handler.lock().unwrap().take().unwrap()().unwrap();
        ppk2.set_device_power(DevicePower::Disabled).unwrap();
        std::process::exit(0);
    })?;
    let mut preamble_detected = false;
    let mut sum = 0f32;
    let mut count = 0;
    let mut report_count = 0;
    // 6. Power on
    let ppk2 = loop {
        let rcv_res = rx.recv_timeout(Duration::from_millis(2000));
        if report_count >= EXPECTED_TESTS {
            break kill.lock().unwrap().take().map(|k| k()).unwrap();
        }
        use MeasurementMatch::*;
        match rcv_res {
            Ok(Match(Measurement { micro_amps, pins })) if preamble_detected => {
                count += 1;
                sum += micro_amps;
                trace!("Last: {:.4} mA. Bits: {:?}", micro_amps / 1000., pins);
            }
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
            Ok(m) => {
                trace!("No preamble detected yet {m:?}");
            }
            Err(RecvTimeoutError::Disconnected) => {
                break kill.lock().unwrap().take().map(|k| k()).unwrap()
            }
            Err(e) => {
                error!("Error receiving data: {e:?}");
                break Err(e)?;
            }
        }
    };
    if let Ok(mut ppk2) = ppk2 {
        // 8. power off
        ppk2.set_device_power(DevicePower::Disabled)?;
    }
    info!("Goodbye!");
    Ok(())
}
