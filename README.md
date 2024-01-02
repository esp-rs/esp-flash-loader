### esp flash loader

A probe-rs flash loader for Espressif chips.

To build the flash loader:

```bash
$ cargo $(CHIP_NAME) # builds the flashing stub
$ target-gen elf target/$(RUST_TARGET)/release/esp-flashloader output/$(CHIP_NAME).yaml --update --name $(CHIP_NAME)-flashloader
```

Example for the updating the `esp32c3` flash algorithm.

```bash
$ cargo esp32c3
$ target-gen elf target/riscv32imc-unknown-none-elf/release/esp-flashloader output/esp32c3.yaml --update --name esp32c3-flashloader
```

## Chip support

| name    | supported |
| ------- | --------- |
| esp32   | N         |
| esp32s2 | Y         |
| esp32s3 | Y         |
| esp32c2 | Y         |
| esp32c3 | Y         |
| esp32c6 | Y         |
| esp32h2 | Y         |

## Adding new chips

1. Add a feature for the chip inside `Cargo.toml`
2. Add a build alias to `.cargo/config.toml`
3. Add the [ROM API linker script](https://github.com/search?q=repo%3Aespressif%2Fesp-idf++path%3A*rom.api.ld&type=code) inside the `ld` directory.
4. Inside the ROM API linker script, add a memory section detailing where the program will be loaded.
    ```c
    MEMORY {
        /* Start 64k into the RAM region */
        IRAM : ORIGIN = 0x40390000, LENGTH = 0x40000
    }
    ```
    It's important to note that the algorithm cannot be loaded at the start of RAM, because probe-rs has a header it loads prior to the algo hence the 64K offset.
    IRAM origin and length can be obtained from esp-hal. Eg: [ESP32-C3 memory map](https://github.com/esp-rs/esp-hal/blob/ff80b69183739d04d1cb154b8232be01c0b26fd9/esp32c3-hal/ld/db-esp32c3-memory.x#L5-L22)
5. Add the following snippet to the `main()` function inside `build.rs`, adapting it for the new chip name.
    ```rust
    #[cfg(feature = "esp32c3")]
    let chip = "esp32c3";
    ```
6. [Define `spiconfig` for your the target in `main.rs`](https://github.com/search?q=repo%3Aespressif%2Fesp-idf+ets_efuse_get_spiconfig+path%3A*c3*&type=code)
7. Follow the instructions above for building
  - It may fail with: `rust-lld: error: undefined symbol: <symbol>`
    - In this case, you need to add the missing method in the ROM API linker script.
      - Eg. ESP32-C2 is missing `esp_rom_spiflash_attach`:
        1. [Search the symbol in esp-idf](https://github.com/search?q=repo%3Aespressif%2Fesp-idf+esp_rom_spiflash_attach+path%3A*c2*&type=code)
        2. Add it to the ROM API linker script: `PROVIDE(esp_rom_spiflash_attach = spi_flash_attach);`
8. Use `target-gen` _without_ the `update` flag to generate a new yaml algorithm.
9. Update the resulting yaml file
   1. Update `name`
   2. Update variants `name`, `type`, `core_access_options` and `memory_map`
      - The first `!Nvm`  block represents the raw flash starting at 0 and up to the maximum supported external flash (check TRM for this, usually in "System and Memory/Features")
      - Next `!Ram` block corresponds to instruction bus for internal SRAM, see Internal Memory Address Mapping of TRM
      - Next `!Ram` block corresponds to data bus for internal SRAM, see Internal Memory Address Mapping of TRM
      - Next `!Nvm` corresponds to instruction bus for external memory, see External Memory Address Mapping of TRM
      - Next `!Nvm` corresponds to data bus for external memory, see External Memory Address Mapping of TRM
   3. Add `load_address` under `flash_algorithms` and assign the IRAM `ORIGIN` value (step 3).
   4. Add `transfer_encoding: Miniz` under `load_address`
9. Upstream the new updates to probe-rs.
