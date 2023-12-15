extern "C" {

    // fn esp_rom_spiflash_write_encrypted_enable();
    // fn esp_rom_spiflash_write_encrypted_disable();
    // fn esp_rom_spiflash_write_encrypted(addr: u32, data: *const u8, len: u32);
    // fn esp_rom_spiflash_config_param();
    // fn esp_rom_spiflash_select_qio_pins();
    // fn esp_rom_spi_flash_auto_sus_res();
    // fn esp_rom_spi_flash_send_resume();
    // fn esp_rom_spi_flash_update_id();
    // fn esp_rom_spiflash_config_clk();
    // fn esp_rom_spiflash_config_readmode();
    // fn esp_rom_spiflash_read_status(/* esp_rom_spiflash_chip_t *spi ,*/ status: *mut u32);
    // fn esp_rom_spiflash_read_statushigh(/* esp_rom_spiflash_chip_t *spi ,*/ status: *mut u32);
    // fn esp_rom_spiflash_write_status(/* esp_rom_spiflash_chip_t *spi ,*/ status: *mut u32);

    fn esp_rom_spiflash_erase_chip() -> i32;
    fn esp_rom_spiflash_erase_block(block_number: u32) -> i32;
    // fn esp_rom_spiflash_erase_sector(sector_number: u32) -> i32;
    /// address (4 byte alignment), data, length
    fn esp_rom_spiflash_write(dest_addr: u32, data: *const u8, len: u32) -> i32;
    /// address (4 byte alignment), data, length
    // fn esp_rom_spiflash_read(src_addr: u32, data: *const u32, len: u32) -> i32;
    fn esp_rom_spiflash_read_user_cmd(status: *mut u32, cmd: u8) -> i32;
    // fn esp_rom_spiflash_unlock() -> i32;
    // fn esp_rom_spiflash_lock(); // can't find in idf defs?
    fn esp_rom_spiflash_attach(config: u32, legacy: bool);

    #[cfg(feature = "esp32c3")]
    fn ets_efuse_get_spiconfig() -> u32;
}

pub fn attach() {
    #[cfg(feature = "esp32c3")]
    let spiconfig: u32 = unsafe { ets_efuse_get_spiconfig() };
    #[cfg(any(feature = "esp32c2", feature = "esp32c6", feature = "esp32h2"))]
    let spiconfig: u32 = 0;

    // TODO: raise CPU frequency

    unsafe { esp_rom_spiflash_attach(spiconfig, false) };
}

pub fn erase_block(adr: u32) -> i32 {
    crate::dprintln!("ERASE @ {}", adr);

    unsafe { esp_rom_spiflash_erase_block(adr / crate::properties::FLASH_BLOCK_SIZE) }
}

pub fn erase_chip() -> i32 {
    unsafe { esp_rom_spiflash_erase_chip() }
}

pub fn write_flash(address: u32, data: &[u8]) -> i32 {
    if data.is_empty() {
        return 0;
    }
    let len = data.len() as u32;
    unsafe { esp_rom_spiflash_write(address, data.as_ptr(), len) }
}

pub fn wait_for_idle() -> i32 {
    const SR_WIP: u32 = 1 << 0;

    let mut status = SR_WIP;
    while status & SR_WIP != 0 {
        let res = unsafe { esp_rom_spiflash_read_user_cmd(&mut status, 0x05) };
        if res != 0 {
            return res;
        }
    }

    0
}
