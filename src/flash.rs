extern "C" {

    // fn esp_rom_spiflash_write_encrypted_enable();
    // fn esp_rom_spiflash_write_encrypted_disable();
    // fn esp_rom_spiflash_write_encrypted(addr: u32, data: *const u8, len: u32);
    // fn esp_rom_spiflash_select_qio_pins();
    // fn esp_rom_spi_flash_auto_sus_res();
    // fn esp_rom_spi_flash_send_resume();
    // fn esp_rom_spi_flash_update_id();
    // fn esp_rom_spiflash_config_clk();
    // fn esp_rom_spiflash_config_readmode();
    // fn esp_rom_spiflash_read_status(/* esp_rom_spiflash_chip_t *spi ,*/ status: *mut u32);
    // fn esp_rom_spiflash_read_statushigh(/* esp_rom_spiflash_chip_t *spi ,*/ status: *mut u32);
    // fn esp_rom_spiflash_write_status(/* esp_rom_spiflash_chip_t *spi ,*/ status: *mut u32);

    #[cfg(not(feature = "esp32s2"))]
    fn esp_rom_spiflash_erase_chip() -> i32;
    fn esp_rom_spiflash_erase_block(block_number: u32) -> i32;
    // fn esp_rom_spiflash_erase_sector(sector_number: u32) -> i32;
    /// address (4 byte alignment), data, length
    fn esp_rom_spiflash_write(dest_addr: u32, data: *const u8, len: u32) -> i32;
    /// address (4 byte alignment), data, length
    fn esp_rom_spiflash_read(src_addr: u32, data: *mut u8, len: u32) -> i32;
    fn esp_rom_spiflash_read_user_cmd(status: *mut u32, cmd: u8) -> i32;
    // fn esp_rom_spiflash_unlock() -> i32;
    // fn esp_rom_spiflash_lock(); // can't find in idf defs?
    fn esp_rom_spiflash_attach(config: u32, legacy: bool);

    fn esp_rom_spiflash_config_param(
        device_id: u32,
        chip_size: u32,
        block_size: u32,
        sector_size: u32,
        page_size: u32,
        status_mask: u32,
    ) -> u32;

    #[cfg(any(
        feature = "esp32",
        feature = "esp32s2",
        feature = "esp32s3",
        feature = "esp32c3",
    ))]
    fn ets_efuse_get_spiconfig() -> u32;
}

pub fn attach() -> i32 {
    #[cfg(any(
        feature = "esp32",
        feature = "esp32s2",
        feature = "esp32s3",
        feature = "esp32c3",
    ))]
    let spiconfig = unsafe { ets_efuse_get_spiconfig() };

    #[cfg(any(feature = "esp32c2", feature = "esp32c6", feature = "esp32h2"))]
    let spiconfig = 0;

    // TODO: raise CPU frequency

    let config_result = unsafe {
        esp_rom_spiflash_config_param(
            0,
            crate::properties::FLASH_SIZE,        // total_size
            crate::properties::FLASH_BLOCK_SIZE,  // block_size
            crate::properties::FLASH_SECTOR_SIZE, // sector_size
            crate::properties::PAGE_SIZE,         // page_size
            crate::properties::FLASH_STATUS_MASK, // status_mask
        )
    };

    if config_result == 0 {
        unsafe { esp_rom_spiflash_attach(spiconfig, false) };
        0
    } else {
        -1
    }
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

pub fn read_flash(address: u32, data: &mut [u8]) -> i32 {
    if data.is_empty() {
        return 0;
    }
    let len = data.len() as u32;
    unsafe { esp_rom_spiflash_read(address, data.as_mut_ptr(), len) }
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

#[cfg(feature = "esp32s2")]
unsafe fn esp_rom_spiflash_erase_chip() -> i32 {
    let res = wait_for_idle();
    if res < 0 {
        return res;
    }

    let cmd_reg: *mut u32 = core::mem::transmute(0x3f40_2000);

    cmd_reg.write_volatile(1 << 22);
    while cmd_reg.read_volatile() != 0 {}

    0
}
