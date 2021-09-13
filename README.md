### esp flash loader

A WIP probe-rs flash loader.

To build the flash loader:

* Check correct `.cargo/config.toml` settings are uncommented.
* Run: 
```bash
cargo build --release # builds the on chip flasher code
target-gen elf target/riscv32imc-unknown-none-elf/release/esp-flashloader > output/esp32c3.yaml # dumps the elf to yaml format for probe-rs
```
