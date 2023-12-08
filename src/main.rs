#![no_std]
#![no_main]

// Define necessary functions for flash loader
//
// These are taken from the [ARM CMSIS-Pack documentation]
//
// [ARM CMSIS-Pack documentation]: https://arm-software.github.io/CMSIS_5/Pack/html/algorithmFunc.html

use panic_never as _;

const FLASH_BLOCK_SIZE: u32 = 65536;

#[cfg(not(any(target_arch = "xtensa", target_arch = "riscv32")))]
compile_error!("specify the target with `--target`");

#[cfg(feature = "log")]
mod log {

    use ufmt::uWrite;
    pub struct Uart;

    impl uWrite for Uart {
        type Error = ();
        fn write_str(&mut self, s: &str) -> Result<(), ()> {
            Ok(for &b in s.as_bytes() {
                unsafe { crate::uart_tx_one_char(b) };
            })
        }
    }
}

#[cfg(feature = "log")]
macro_rules! dprintln {
    () => {
        ufmt::uwriteln!(crate::log::Uart, "").ok()
    };
    ($fmt:literal) => {
        ufmt::uwriteln!(crate::log::Uart, $fmt).ok()
    };
    ($fmt:literal, $($arg:tt)*) => {
        ufmt::uwriteln!(crate::log::Uart, $fmt, $($arg)*).ok()
    };
}

#[cfg(not(feature = "log"))]
#[macro_export]
macro_rules! dprintln {
    () => {};
    ($fmt:expr) => {};
    ($fmt:expr, $($arg:tt)*) => {};
}

#[allow(unused)]
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
    fn esp_rom_spiflash_erase_sector(sector_number: u32) -> i32;
    /// address (4 byte alignment), data, length
    fn esp_rom_spiflash_write(dest_addr: u32, data: *const u8, len: u32) -> i32;
    /// address (4 byte alignment), data, length
    fn esp_rom_spiflash_read(src_addr: u32, data: *mut u32, len: u32) -> i32;
    fn esp_rom_spiflash_read_user_cmd(status: *mut u32, cmd: u8) -> i32;
    fn esp_rom_spiflash_unlock() -> i32;
    // fn esp_rom_spiflash_lock(); // can't find in idf defs?
    fn esp_rom_spiflash_attach(config: u32, legacy: bool);

    fn uart_tx_one_char(byte: u8);

    fn ets_efuse_get_spiconfig() -> u32;
}

unsafe fn wait_for_idle() -> i32 {
    const SR_WIP: u32 = 1 << 0;

    let mut status = SR_WIP;
    while status & SR_WIP != 0 {
        let res = esp_rom_spiflash_read_user_cmd(&mut status, 0x05);
        if res != 0 {
            return res;
        }
    }

    0
}

unsafe fn pad_buffer(adr: u32, sz: u32, buf: *mut u8, block_sz: u32) -> i32 {
    let mut src = adr + sz;
    let mut dst = buf.add(sz as usize);
    let mut remaining = block_sz - sz;

    let skip_bytes = src as usize % 4;
    if skip_bytes != 0 {
        let mut tmp = 0;

        let res = esp_rom_spiflash_read(src & 0x3, &mut tmp, 4);
        if res != 0 {
            return res;
        }

        match skip_bytes {
            1 => {
                *dst = (tmp >> 8) as u8;
                dst = dst.add(1);
                *dst = (tmp >> 16) as u8;
                dst = dst.add(1);
                *dst = (tmp >> 24) as u8;
                dst = dst.add(1);
            }
            2 => {
                *dst = (tmp >> 16) as u8;
                dst = dst.add(1);
                *dst = (tmp >> 24) as u8;
                dst = dst.add(1);
            }
            3 => {
                *dst = (tmp >> 24) as u8;
                dst = dst.add(1);
            }
            _ => {}
        }

        src += skip_bytes as u32;
        remaining += skip_bytes as u32;
    }

    esp_rom_spiflash_read(src, dst.cast(), remaining)
}

unsafe fn compare(mut a: *const u8, mut b: *const u8, len: u32) -> bool {
    let end = a.add(len as usize);

    while a < end {
        if *a != *b {
            return false;
        }
        a = a.add(1);
        b = b.add(1);
    }

    true
}

unsafe fn is_empty(mut a: *const u8, len: u32) -> bool {
    let end = a.add(len as usize);

    while a < end {
        if *a != 0xFF {
            return false;
        }
        a = a.add(1);
    }

    true
}

enum VerifyResult {
    Same,
    Different,
    Empty,
}

unsafe fn verify_buffer(adr: u32, sz: u32, buf: *const u8) -> Result<VerifyResult, i32> {
    let mut buf = buf;
    let mut adr = adr;
    let end = adr + sz;

    let mut result = None;

    let mut readback = core::mem::MaybeUninit::<[u8; 256]>::uninit();
    while adr < end {
        let res = esp_rom_spiflash_read(adr, readback.as_mut_ptr().cast(), 256);
        if res != 0 {
            return Err(res);
        }

        match result {
            Some(VerifyResult::Empty) => {
                if !is_empty(readback.as_ptr().cast(), 256) {
                    return Ok(VerifyResult::Different);
                }
            }
            Some(VerifyResult::Different) => {}
            Some(VerifyResult::Same) => {
                if !compare(buf, readback.as_ptr().cast(), 256) {
                    return Ok(VerifyResult::Different);
                }
            }
            None => {
                if is_empty(readback.as_ptr().cast(), 256) {
                    result = Some(VerifyResult::Empty);
                } else if compare(buf, readback.as_ptr().cast(), 256) {
                    result = Some(VerifyResult::Same);
                } else {
                    return Ok(VerifyResult::Different);
                }
            }
        }

        buf = buf.add(256);
        adr += 256;
    }

    Ok(VerifyResult::Same)
}

/// Setup the device for the
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn Init(_adr: u32, _clk: u32, _fnc: u32) -> i32 {
    static mut INITD: bool = false;

    if !INITD {
        dprintln!("INIT");

        #[cfg(feature = "esp32c3")]
        let spiconfig: u32 = ets_efuse_get_spiconfig();
        #[cfg(any(feature = "esp32c2", feature = "esp32c6", feature = "esp32h2"))]
        let spiconfig: u32 = 0;

        esp_rom_spiflash_attach(spiconfig, false);

        INITD = true;
    }

    0
}

/// Erase the sector at the given address in flash
///
/// Returns 0 on success, 1 on failure.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn EraseSector(_adr: u32) -> i32 {
    0
}

#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn EraseChip() -> i32 {
    0
}

#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ProgramPage(adr: u32, sz: u32, buf: *const u8) -> i32 {
    let buf_addr: u32 = buf as *const _ as _;
    if buf_addr % 4 != 0 {
        dprintln!("ERROR buf not word aligned");
        return 0xAA;
    }

    let block_sz = if sz <= 4096 { 4096 } else { 65536 };

    dprintln!("PROGRAM {} bytes @ {}", sz, adr);

    let mut res = pad_buffer(adr, sz, buf.cast_mut(), block_sz);
    let mut erase = false;

    if res == 0 {
        match verify_buffer(adr, block_sz, buf) {
            Ok(VerifyResult::Same) => return 0,
            Ok(VerifyResult::Different) => erase = true,
            Ok(VerifyResult::Empty) => erase = false,
            Err(err) => return err,
        };
    }

    if res == 0 && erase {
        res = if block_sz == 4096 {
            esp_rom_spiflash_erase_sector(adr / 4096)
        } else {
            esp_rom_spiflash_erase_block(adr / 65536)
        };
    }

    if res == 0 {
        res = esp_rom_spiflash_write(adr, buf, block_sz);
    }

    res
}

#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn UnInit(_fnc: u32) -> i32 {
    wait_for_idle()
}

#[allow(non_upper_case_globals)]
#[no_mangle]
#[used]
#[link_section = "DeviceData"]
pub static FlashDevice: FlashDeviceDescription = FlashDeviceDescription {
    vers: 0x0001,
    dev_name: [0u8; 128],
    dev_type: 5,
    dev_addr: 0x0,
    device_size: 0x4000000, /* Max of 64MB */
    // TODO change per variant?
    page_size: FLASH_BLOCK_SIZE,
    _reserved: 0,
    empty: 0xFF,
    program_time_out: 1000,
    erase_time_out: 2000,
    flash_sectors: sectors(),
};

const fn sectors() -> [FlashSector; 512] {
    const SECTOR_END: FlashSector = FlashSector {
        size: 0xffff_ffff,
        address: 0xffff_ffff,
    };

    let mut sectors = [FlashSector::default(); 512];

    sectors[0] = FlashSector {
        size: 0x10000, // 64k
        address: 0x0,
    };
    sectors[1] = SECTOR_END;

    sectors
}

#[repr(C)]
pub struct FlashDeviceDescription {
    vers: u16,
    dev_name: [u8; 128],
    dev_type: u16,
    dev_addr: u32,
    device_size: u32,
    page_size: u32,
    _reserved: u32,
    empty: u8,
    program_time_out: u32,
    erase_time_out: u32,
    flash_sectors: [FlashSector; 512],
}

#[repr(C)]
#[derive(Copy, Clone)]
struct FlashSector {
    size: u32,
    address: u32,
}

impl FlashSector {
    const fn default() -> Self {
        FlashSector {
            size: 0,
            address: 0,
        }
    }
}
