# Powertest

A embedded power benchmarking tool, based on [`ppk2-rs`][ppk2-rs].

Uses the [Nordic Power Profiler Kit II][ppk2] to measure current usage during tests run by [`defmt-test`][defmt-test]. See [fw-example] for an example of its use.

## How it works
Powertest uses the [Nordic Power Profiler Kit II][ppk2] (PPK2) to control device power and to measure its current usage. To get the most accurate results, `powertest` doesn't rely on info that is output by the debugger to determine what it should measure. Instead, the device can pull a pin low to signal that it is running a test, and push it high to signal that no test is currently running. 

`powertest` is used as a Cargo runner, and takes the ELF that contains the test binary as an argument. It analyzes the ELF file to find out how many tests are going to be run by the device. Then, it loads the binary onto the chip using [`probe-rs`][probe-rs] and starts fetching measurements from the PPK2. These measurements contain data about current usage as well as on levels of the pins in the logic port. To determine when a test starts running, `powertest` listens for a high-to-low transition on pin `D0` of the PPK2. It collects current usage data until it detects a low-to-high transition, at which point it outputs the average current use. It does this until as many high-to-low and low-to-high transitions are detected as there are tests defined in the ELF. 


[ppk2]: https://www.nordicsemi.com/Products/Development-hardware/Power-Profiler-Kit-2 
[defmt-test]: https://github.com/knurling-rs/defmt
[ppk2-rs]: https://github.com/hdoordt/ppk2-rs
[fw-example]: https://github.com/hdoordt/powertest/blob/main/fw-example
[probe-rs]: https://github.com/probe-rs/probe-rs
