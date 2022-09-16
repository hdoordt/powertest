# Powertest example firmware
Contains an example test that can be run on an nRF52840DK

## Pin connections

If you just want to try out the tool:

| PPK2 Logic port pin | nRF52840DK pin |
|---------------------|----------------|
| VCC                 | VDD            |
| GND                 | GND            |
| D0                  | P0.03          |

## Run
In one terminal window, start `powertest`. Then, in another terminal window, run
```bash
cargo test --test power
```

You should see power measurement reports coming in as the tests are being run.
