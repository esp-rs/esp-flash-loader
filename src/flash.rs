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
        feature = "esp32c61",
        feature = "esp32h2",
        feature = "esp32p4"
    ))]
    let spiconfig = 0;

    unsafe { esp_rom_spiflash_attach(spiconfig, false) };

    #[cfg(feature = "esp32s3")]
    if unsafe { ets_efuse_flash_octal_mode() } {
        init_ospi_funcs();
    }

    let config_result = unsafe {
        esp_rom_spiflash_config_param(
            0,
            crate::properties::MAX_FLASH_SIZE,    // total_size
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

fn read_spi_reg(reg: u32) -> u32 {
    unsafe { (reg as *mut u32).read_volatile() }
}

fn write_spi_reg(reg: u32, value: u32) {
    unsafe { (reg as *mut u32).write_volatile(value) }
}

#[allow(unused)]
pub struct MemSpi {
    pub base: u32,
    pub cmd: u32,
    pub addr: u32,
    pub ctrl: u32,
    pub user: u32,
    pub user1: u32,
    pub user2: u32,
    pub miso_dlen: u32,
    pub data_buf_0: u32,
}

#[allow(unused)]
impl MemSpi {
    fn cmd(&self) -> u32 {
        self.base as u32 | self.cmd as u32
    }

    fn addr(&self) -> u32 {
        self.base as u32 | self.addr as u32
    }

    fn ctrl(&self) -> u32 {
        self.base as u32 | self.ctrl as u32
    }

    fn user(&self) -> u32 {
        self.base as u32 | self.user as u32
    }

    fn user1(&self) -> u32 {
        self.base as u32 | self.user1 as u32
    }

    fn user2(&self) -> u32 {
        self.base as u32 | self.user2 as u32
    }

    fn miso_dlen(&self) -> u32 {
        self.base as u32 | self.miso_dlen as u32
    }

    fn data_buf_0(&self) -> u32 {
        self.base as u32 | self.data_buf_0 as u32
    }
}

fn spi_send_command(command: u32, len: u32) -> u32 {
    let regs = crate::chip::MEM_SPI;

    // Save registers
    let old_user_reg = read_spi_reg(regs.user());
    let old_user2_reg = read_spi_reg(regs.user2());

    // user register
    const USER_MISO: u32 = 1 << 28;
    const USER_COMMAND: u32 = 1 << 31;

    // user2 register
    const USER_COMMAND_BITLEN: u32 = 28;

    // miso dlen register
    const MISO_BITLEN: u32 = 0;

    // cmd register
    const USER_CMD: u32 = 1 << 18;

    write_spi_reg(regs.user(), old_user_reg | USER_COMMAND | USER_MISO);
    write_spi_reg(regs.user2(), (7 << USER_COMMAND_BITLEN) | command);
    write_spi_reg(regs.addr(), 0);

    write_spi_reg(regs.miso_dlen(), (len.saturating_sub(1)) << MISO_BITLEN);
    write_spi_reg(regs.data_buf_0(), 0);

    // Execute read
    write_spi_reg(regs.cmd(), USER_CMD);
    while read_spi_reg(regs.cmd()) & USER_CMD != 0 {}

    // Read result
    let value = read_spi_reg(regs.data_buf_0());

    // Restore registers
    write_spi_reg(regs.user(), old_user_reg);
    write_spi_reg(regs.user2(), old_user2_reg);

    value & ((1 << len) - 1)
}

pub fn get_flash_size() -> i32 {
    const RDID: u32 = 0x9F;
    let id = spi_send_command(RDID, 24);

    const KB: i32 = 1024;
    const MB: i32 = 1024 * KB;

    // https://github.com/espressif/esptool/blob/8363cae8eca42ec70e26edfe4d1727549d6ce578/esptool/cmds.py#L55-L98
    let [manufacturer, _, _, _] = id.to_le_bytes();
    const ADESTO_VENDOR_ID: u8 = 0x1F;
    if manufacturer == ADESTO_VENDOR_ID {
        let [_, capacity, _, _] = id.to_le_bytes();
        match capacity & 0x1F {
            0x04 => 512 * KB,
            0x05 => 1 * MB,
            0x06 => 2 * MB,
            0x07 => 4 * MB,
            0x08 => 8 * MB,
            0x09 => 16 * MB,
            _ => -1,
        }
    } else {
        let [_, _, capacity, _] = id.to_le_bytes();
        match capacity {
            0x12 => 256 * KB,
            0x13 => 512 * KB,
            0x14 => 1 * MB,
            0x15 => 2 * MB,
            0x16 => 4 * MB,
            0x17 => 8 * MB,
            0x18 => 16 * MB,
            0x19 => 32 * MB,
            0x1A => 64 * MB,
            0x1B => 128 * MB,
            0x1C => 256 * MB,
            0x20 => 64 * MB,
            0x21 => 128 * MB,
            0x22 => 256 * MB,
            0x32 => 256 * KB,
            0x33 => 512 * KB,
            0x34 => 1 * MB,
            0x35 => 2 * MB,
            0x36 => 4 * MB,
            0x37 => 8 * MB,
            0x38 => 16 * MB,
            0x39 => 32 * MB,
            0x3A => 64 * MB,
            _ => -1,
        }
    }
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
