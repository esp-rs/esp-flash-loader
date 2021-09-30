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

## Chip support

|name   |supported|
|-------|---------|
|esp32c3|   Y     |

## Adding new chips

1. Add a feature for the chip inside `Cargo.toml`
2. Add the ROM API linker script inside the `ld` directory.
3. Inside the ROM API linker script, add a memory section detailing where the program will be loaded.
    ```c
    MEMORY {
        /* Start 64k into the RAM region */
        IRAM : ORIGIN = 0x40390000, LENGTH = 0x40000
    }
    ```
    It's important to note that the algorithm cannot be loaded at the start of RAM, because probe-rs has a header it loads prior to the algo hence the 64K offset.
4. Add the following snippet to the bottom of the `main()` function inside `build.rs`, adapting it for the new chip name.
    ```rust
    #[cfg(feature = "esp32c3")]
    {
        fs::copy("ld/esp32c3.x", out_dir.join("esp32c3.x")).unwrap();
        println!("cargo:rerun-if-changed=ld/esp32c3.x");
        println!("cargo:rustc-link-arg=-Tld/esp32c3.x");
    }
    ```
5. Follow the instructions above for building
6. Use `target-gen` _without_ the `update` flag to generate a new yaml algorithm.
7. merge the new flash algorithm into the the main `esp32.yaml`
8. Upstream the new updates to probe-rs.