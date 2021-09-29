### esp flash loader

A probe-rs flash loader for Espressif chips.

To build the flash loader:

```bash
$ cargo build --release --feature $(CHIP_NAME) --target $(RUST_TARGET) # builds the flashing stub
$ target-gen elf target/$(RUST_TARGET)/release/esp-flashloader > output/$(CHIP_NAME).yaml # dumps the elf to yaml format for probe-rs
```

Example for the `esp32c3`
```bash
$ cargo build --release --features esp32c3 --target riscv32imc-unknown-none-elf
$ target-gen elf target/riscv32imc-unknown-none-elf/release/esp-flashloader > output/esp32c3.yaml
```