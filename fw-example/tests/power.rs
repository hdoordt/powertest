#![no_std]
#![no_main]

use powertest_fw as _; // memory layout + panic handler

// See https://crates.io/crates/defmt-test/0.3.0 for more documentation (e.g. about the 'state'
// feature)
#[defmt_test::tests]
mod tests {
    use hal::{gpio::Level, pac, prelude::OutputPin};
    use nrf52840_hal as hal;
    use defmt::assert;
    use nrf52840_hal::gpio::{PushPull, Output, Pin};

    #[init]
    fn init() -> Pin<Output<PushPull>> {
        let p = pac::Peripherals::take().unwrap();
        let port0 = hal::gpio::p0::Parts::new(p.P0);
        // Initially set test signal pin to high. 
        // Powertest will start measuring on the first
        // high-to-low transition of the pin.
        let test_signal_pin = port0.p0_03.into_push_pull_output(Level::High).degrade();
        cortex_m::asm::delay(64_000_000); 
        test_signal_pin
    }

    #[before_each]
    fn before_each(test_signal_pin: &mut Pin<Output<PushPull>>) {
        // Set pin low to signal that a test has started
        test_signal_pin.set_low().unwrap();
        // As this delay affects measurements,
        // it should be as short as possible, though long enough
        // for powertest to detect it.
        cortex_m::asm::delay(320);
    }

    #[after_each]
    fn after_each(test_signal_pin: &mut Pin<Output<PushPull>>) {
        // Set pin high to signal that a test has stopped
        test_signal_pin.set_high().unwrap();
        // Measurements are ignored if pin is high,
        // so the length of this delay does not affect
        // measurement data
        cortex_m::asm::delay(64000000);
    }

    #[test]
    fn it_works() {
        assert!(true);
    }

    #[test]
    fn it_works_even_better() {
        assert!(true);
    }

    #[test]
    fn it_works_very_well() {
        assert!(true);
    }

}
