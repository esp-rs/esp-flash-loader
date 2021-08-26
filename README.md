### esp flash loader

A WIP probe-rs flash loader.

To build the flash loader:

* Check correct `.cargo/config.toml` settings are uncommented.
* Run: 
```bash
cargo build --release # builds the on chip flasher code
target-gen elf target/riscv32imc-unknown-none-elf/release/esp-flashloader > output/esp32c3.yaml # dumps the elf to yaml format for probe-rs
```

To Run the program standalone:

* Check correct `.cargo/config.toml` settings are uncommented.
* Run `cargo build --release --features standalone`.
* Run `esptool.py --chip esp32c3 elf2image --flash_mode=dio -o esp32c3.bin target/riscv32imc-unknown-none-elf/release/esp-flashloader`.
* Run `esptool.py --chip esp32c3 -p /dev/ttyUSB0 --after hard_reset write_flash 0x0 esp32c3.bin` to flash it to the board.