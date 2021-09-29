### esp flash loader

A probe-rs flash loader for Espressif chips.

To build the flash loader:

```bash
$ cargo build --release --feature $(CHIP_NAME) --target $(RUST_TARGET) # builds the flashing stub
$ target-gen elf target/$(RUST_TARGET)/release/esp-flashloader output/esp32.yaml --update --name $(CHIP_NAME)-flashloader
```

Example for the updating the `esp32c3` flash algorithm.
```bash
$ cargo build --release --features esp32c3 --target riscv32imc-unknown-none-elf
$ target-gen elf target/riscv32imc-unknown-none-elf/release/esp-flashloader output/esp32.yaml --update --name esp32c3-flashloader
```