# Powertest

A embedded power benchmarking tool, based on [`ppk2-rs`].

Uses the [Nordic Power Profiler Kit II](https://www.nordicsemi.com/Products/Development-hardware/Power-Profiler-Kit-2) to measure current usage during tests run by [`defmt-test`]. See [fw-example] for an example of its use.


[`defmt-test`]: https://github.com/knurling-rs/defmt
[`ppk2-rs`]: https://github.com/hdoordt/ppk2-rs
[fw-example]: https://github.com/hdoordt/powertest/blob/main/fw-example