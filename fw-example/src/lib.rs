#![no_main]
#![no_std]

use defmt_rtt as _; // global logger

use hal::{gpio::Level, pac};
use nrf52840_hal as hal; // memory layout

use panic_probe as _;

// same panicking *behavior* as `panic-probe` but doesn't print a panic message
// this prevents the panic message being printed *twice* when `defmt::panic` is invoked
#[defmt::panic_handler]
fn panic() -> ! {
    let p = unsafe { pac::Peripherals::steal() };
    let port0 = hal::gpio::p0::Parts::new(p.P0);
    let _test_signal_pin = port0.p0_03.into_push_pull_output(Level::High).degrade();
    cortex_m::asm::udf()
}

/// Terminates the application and makes `probe-run` exit with exit-code = 0
pub fn exit() -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}
