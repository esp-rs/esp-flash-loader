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

#[cfg(feature = "esp32s3")]
#[allow(non_camel_case_types)]
mod s3 {
    type spi_flash_func_t = unsafe extern "C" fn();
    type spi_flash_op_t = unsafe extern "C" fn() -> i32;
    type spi_flash_erase_t = unsafe extern "C" fn(u32) -> i32;
    type spi_flash_rd_t = unsafe extern "C" fn(u32, *mut (), i32) -> i32;
    type spi_flash_wr_t = unsafe extern "C" fn(u32, *const u32, i32) -> i32;
    type spi_flash_ewr_t = unsafe extern "C" fn(u32, *const (), u32) -> i32;
    type spi_flash_wren_t = unsafe extern "C" fn(*mut ()) -> i32;
    type spi_flash_erase_area_t = unsafe extern "C" fn(u32, u32) -> i32;

    #[repr(C)]
    pub struct spiflash_legacy_funcs_t {
        pub pp_addr_bit_len: u8,
        pub se_addr_bit_len: u8,
        pub be_addr_bit_len: u8,
        pub rd_addr_bit_len: u8,
        pub read_sub_len: u32,
        pub write_sub_len: u32,
        pub unlock: Option<spi_flash_op_t>,
        pub erase_sector: Option<spi_flash_erase_t>,
        pub erase_block: Option<spi_flash_erase_t>,
        pub read: Option<spi_flash_rd_t>,
        pub write: Option<spi_flash_wr_t>,
        pub encrypt_write: Option<spi_flash_ewr_t>,
        pub check_sus: Option<spi_flash_func_t>,
        pub wren: Option<spi_flash_wren_t>,
        pub wait_idle: Option<spi_flash_op_t>,
        pub erase_area: Option<spi_flash_erase_area_t>,
    }
}

#[cfg(feature = "esp32s3")]
use s3::*;

#[cfg(feature = "esp32s3")]
extern "C" {
    static mut rom_spiflash_legacy_funcs: *const spiflash_legacy_funcs_t;
    static mut rom_spiflash_legacy_data: *mut ();

    fn ets_efuse_flash_octal_mode() -> bool;

    fn esp_rom_opiflash_wait_idle() -> i32;
    fn esp_rom_opiflash_erase_block_64k(addr: u32) -> i32;
    fn esp_rom_opiflash_erase_sector(addr: u32) -> i32;
    fn esp_rom_opiflash_read(addr: u32, buf: *mut (), len: i32) -> i32;
    fn esp_rom_opiflash_write(addr: u32, data: *const u32, len: i32) -> i32;
    fn esp_rom_opiflash_wren(p: *mut ()) -> i32;
    fn esp_rom_opiflash_erase_area(start_addr: u32, end_addr: u32) -> i32;
}

pub fn attach() -> i32 {
    #[cfg(any(
        feature = "esp32",
        feature = "esp32s2",
        feature = "esp32s3",
        feature = "esp32c3",
    ))]
    let spiconfig = unsafe { ets_efuse_get_spiconfig() };

    #[cfg(any(
        feature = "esp32c2",
        feature = "esp32c5",
        feature = "esp32c6",
        feature = "esp32h2"
    ))]
    let spiconfig = 0;

    // TODO: raise CPU frequency

    unsafe { esp_rom_spiflash_attach(spiconfig, false) };

    #[cfg(feature = "esp32s3")]
    if unsafe { ets_efuse_flash_octal_mode() } {
        init_ospi_funcs();
    } else {
        // For some reason, the default pointers are not set on boot. I'm not sure if
        // probe-rs does something wrong, or the ROM bootloader doesn't initialize memory properly under some conditions.
        // Leaving these uninitialized ends up with a division by zero exception. These addresses have been mined out of
        // the ROM elf, these supposed to be the default values of these variables.
        unsafe {
            rom_spiflash_legacy_funcs = 0x3FCEF670 as _;
            rom_spiflash_legacy_data = 0x3FCEF6A4 as _;
        }
    }

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

#[cfg(feature = "esp32s3")]
fn init_ospi_funcs() {
    static FUNCS: spiflash_legacy_funcs_t = spiflash_legacy_funcs_t {
        pp_addr_bit_len: 24,
        se_addr_bit_len: 24,
        be_addr_bit_len: 24,
        rd_addr_bit_len: 24,
        read_sub_len: 16,
        write_sub_len: 32,
        unlock: Some(esp_rom_opiflash_wait_idle),
        erase_block: Some(esp_rom_opiflash_erase_block_64k),
        erase_sector: Some(esp_rom_opiflash_erase_sector),
        read: Some(esp_rom_opiflash_read),
        write: Some(esp_rom_opiflash_write),
        encrypt_write: None,
        check_sus: None,
        wait_idle: Some(esp_rom_opiflash_wait_idle),
        wren: Some(esp_rom_opiflash_wren),
        erase_area: Some(esp_rom_opiflash_erase_area),
    };

    unsafe {
        let funcs_iram = &raw const FUNCS;
        rom_spiflash_legacy_funcs =
            ((funcs_iram as usize) - 0x4038_0400 + 0x3FC9_0400) as *const spiflash_legacy_funcs_t;
    }
}
