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

    // fn esp_rom_opiflash_wait_idle() -> i32;
    // fn esp_rom_opiflash_erase_block_64k(addr: u32) -> i32;
    // fn esp_rom_opiflash_erase_sector(addr: u32) -> i32;
    // fn esp_rom_opiflash_read(addr: u32, buf: *mut (), len: i32) -> i32;
    // fn esp_rom_opiflash_write(addr: u32, data: *const u32, len: i32) -> i32;
    // fn esp_rom_opiflash_wren(p: *mut ()) -> i32;
    // fn esp_rom_opiflash_erase_area(start_addr: u32, end_addr: u32) -> i32;
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
        feature = "esp32h21",
        feature = "esp32h4",
        feature = "esp32p4",
        feature = "esp32s31"
    ))]
    let spiconfig = 0;

    unsafe { esp_rom_spiflash_attach(spiconfig, false) };

    #[cfg(feature = "esp32s3")]
    if unsafe { ets_efuse_flash_octal_mode() } {
        crate::dprintln!("Octal mode");
        init_ospi_funcs();
    }

    // Enable the command phase for SPI1 user transactions. In the reset-run case
    // (how probe-rs loads the stub) this bit is not set by default, so ROM
    // write/erase commands (incl. write-enable) would go out without a command
    // byte and silently no-op, leaving the old firmware in place.
    const USER_COMMAND: u32 = 1 << 31;
    let user_reg = crate::chip::MEM_SPI.user();
    write_spi_reg(user_reg, read_spi_reg(user_reg) | USER_COMMAND);

    let config_result = unsafe {
        esp_rom_spiflash_config_param(
            0,
            flash_size(),                         // total_size (detected, capped)
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

    // The ROM write needs a word-aligned length; pad a sub-word tail with 0xFF.
    let aligned_len = data.len() & !3;
    if aligned_len > 0 {
        let res = unsafe { esp_rom_spiflash_write(address, data.as_ptr(), aligned_len as u32) };
        if res != 0 {
            return res;
        }
    }

    let tail_len = data.len() & 3;
    if tail_len == 0 {
        return 0;
    }

    let mut bytes = [0xFFu8; 4];
    let mut i = 0;
    while i < tail_len {
        unsafe { *bytes.as_mut_ptr().add(i) = *data.as_ptr().add(aligned_len + i) };
        i += 1;
    }
    // The ROM reads the source in words, so it must be word-aligned.
    let word = u32::from_ne_bytes(bytes);
    unsafe {
        esp_rom_spiflash_write(
            address + aligned_len as u32,
            (&raw const word).cast::<u8>(),
            4,
        )
    }
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

fn read_flash_id() -> u32 {
    const RDID: u32 = 0x9F;
    spi_send_command(RDID, 24)
}

/// Flash size in bytes from a JEDEC RDID response, or `None` if unknown.
fn decode_flash_size(id: u32) -> Option<u32> {
    const KB: u32 = 1024;
    const MB: u32 = 1024 * KB;

    // https://github.com/espressif/esptool/blob/8363cae8eca42ec70e26edfe4d1727549d6ce578/esptool/cmds.py#L55-L98
    let [manufacturer, _, _, _] = id.to_le_bytes();
    const ADESTO_VENDOR_ID: u8 = 0x1F;
    if manufacturer == ADESTO_VENDOR_ID {
        let [_, capacity, _, _] = id.to_le_bytes();
        match capacity & 0x1F {
            0x04 => Some(512 * KB),
            0x05 => Some(1 * MB),
            0x06 => Some(2 * MB),
            0x07 => Some(4 * MB),
            0x08 => Some(8 * MB),
            0x09 => Some(16 * MB),
            _ => None,
        }
    } else {
        let [_, _, capacity, _] = id.to_le_bytes();
        match capacity {
            0x12 => Some(256 * KB),
            0x13 => Some(512 * KB),
            0x14 => Some(1 * MB),
            0x15 => Some(2 * MB),
            0x16 => Some(4 * MB),
            0x17 => Some(8 * MB),
            0x18 => Some(16 * MB),
            0x19 => Some(32 * MB),
            0x1A => Some(64 * MB),
            0x1B => Some(128 * MB),
            0x1C => Some(256 * MB),
            0x20 => Some(64 * MB),
            0x21 => Some(128 * MB),
            0x22 => Some(256 * MB),
            0x32 => Some(256 * KB),
            0x33 => Some(512 * KB),
            0x34 => Some(1 * MB),
            0x35 => Some(2 * MB),
            0x36 => Some(4 * MB),
            0x37 => Some(8 * MB),
            0x38 => Some(16 * MB),
            0x39 => Some(32 * MB),
            0x3A => Some(64 * MB),
            _ => None,
        }
    }
}

/// Detected flash size, capped to `MAX_FLASH_SIZE`. Unknown/oversized parts use the cap.
pub fn flash_size() -> u32 {
    let max = crate::properties::MAX_FLASH_SIZE;
    match decode_flash_size(read_flash_id()) {
        Some(size) if size > max => {
            crate::dprintln!("Flash size {} exceeds cap {}, capping", size, max);
            max
        }
        Some(size) => size,
        None => {
            crate::dprintln!("Unknown flash ID, assuming cap {}", max);
            max
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
        unlock: None,
        erase_block: None,
        erase_sector: None,
        read: None,
        write: None,
        encrypt_write: None,
        check_sus: None,
        wait_idle: None,
        wren: None,
        erase_area: None,
    };

    unsafe {
        let funcs_iram = &raw const FUNCS;
        rom_spiflash_legacy_funcs =
            ((funcs_iram as usize) - 0x4038_0400 + 0x3FC9_0400) as *const spiflash_legacy_funcs_t;
    }
}
