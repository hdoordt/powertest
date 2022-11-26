# Powertest example firmware
Contains an example test that can be run on an nRF52840DK

## Example

The interesting stuff happens in [tests/power.rs](https://github.com/hdoordt/powertest/blob/main/fw-example/tests/power.rs). You can use this example to set up your own tests.

## Pin connections

If you just want to try out the tool:

| PPK2 Logic port pin | nRF52840DK pin |
|---------------------|----------------|
| VCC                 | VDD            |
| GND                 | GND            |
| D0                  | P0.03          |

## Run
Make sure you've installed powertest:
```bash
cargo install powertest
```

Or, if you want to install a local version, you can run:

```bash
cargo install --path /path/to/powertest
```

To run the test:

```bash
cargo test --test power
```

You should see power measurement reports coming in as the tests are being run.
