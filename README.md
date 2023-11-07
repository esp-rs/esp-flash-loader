### esp flash loader

A probe-rs flash loader for Espressif chips.

To build the flash loader:

```bash
$ cargo build --release --feature $(CHIP_NAME) --target $(RUST_TARGET) # builds the flashing stub
$ target-gen elf target/$(RUST_TARGET)/release/esp-flashloader output/$(CHIP_NAME).yaml --update --name $(CHIP_NAME)-flashloader
```

Example for the updating the `esp32c3` flash algorithm.

```bash
$ cargo build --release --features esp32c3 --target riscv32imc-unknown-none-elf
$ target-gen elf target/riscv32imc-unknown-none-elf/release/esp-flashloader output/esp32c3.yaml --update --name esp32c3-flashloader
```

## Chip support

| name    | supported |
| ------- | --------- |
| esp32c2 | Y         |
| esp32c3 | Y         |
| esp32c6 | Y         |
| esp32h2 | Y         |

## Adding new chips

1. Add a feature for the chip inside `Cargo.toml`
2. Add the [ROM API linker script](https://github.com/search?q=repo%3Aespressif%2Fesp-idf++path%3A*rom.api.ld&type=code) inside the `ld` directory.
3. Inside the ROM API linker script, add a memory section detailing where the program will be loaded.
    ```c
    MEMORY {
        /* Start 64k into the RAM region */
        IRAM : ORIGIN = 0x40390000, LENGTH = 0x40000
    }
    ```
    It's important to note that the algorithm cannot be loaded at the start of RAM, because probe-rs has a header it loads prior to the algo hence the 64K offset.
    IRAM origin and length can be obtained from esp-hal. Eg: [ESP32-C3 memory map](https://github.com/esp-rs/esp-hal/blob/ff80b69183739d04d1cb154b8232be01c0b26fd9/esp32c3-hal/ld/db-esp32c3-memory.x#L5-L22)
4. Add the following snippet to the `main()` function inside `build.rs`, adapting it for the new chip name.
    ```rust
    #[cfg(feature = "esp32c3")]
    let chip = "esp32c3";
    ```
5. [Define `spiconfig` for your the target in `main.rs`](https://github.com/search?q=repo%3Aespressif%2Fesp-idf+ets_efuse_get_spiconfig+path%3A*c3*&type=code)
6. Follow the instructions above for building
  - It may fail with: `rust-lld: error: undefined symbol: <symbol>`
    - In this case, you need to add the missing method in the ROM API linker script.
      - Eg. ESP32-C2 is missing `esp_rom_spiflash_attach`:
        1. [Search the symbol in esp-idf](https://github.com/search?q=repo%3Aespressif%2Fesp-idf+esp_rom_spiflash_attach+path%3A*c2*&type=code)
        2. Add it to the ROM API linker script: `PROVIDE(esp_rom_spiflash_attach = spi_flash_attach);`
7. Use `target-gen` _without_ the `update` flag to generate a new yaml algorithm.
8. Update the resulting yaml file
   1. Update `name`
   2. Update variants `name`, `type`, `core_access_options` and `memory_map`
      - The first `!Nvm`  block represents the raw flash starting at 0 and up to the maximum supported external flash (check TRM for this, usually in "System and Memory/Features")
      - Next `!Ram` block corresponds to IRAM, it starts at ORIGIN + LENGTH (values defined in step 3) and ends at the end of the IRAM section(check target memory map)
      - Next `!Ram` corresponds to DRAM, use the start and end value of the memory map
      - Next `!Nvm` corresponds to IROM, use the start and end value of the memory map
      - Next `!Nvm` corresponds to DROM, use the start and end value of the memory map
   3. Add `load_address` under `flash_algorithms` and assing the IRAM `ORIGIN` value (step 3).
9.  Merge the new flash algorithm into the the main `esp32c3.yaml`
10. Upstream the new updates to probe-rs.
