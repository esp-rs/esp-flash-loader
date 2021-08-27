#![no_std]
#![no_main]

#![feature(asm)]

// Define necessary functions for flash loader
//
// These are taken from the [ARM CMSIS-Pack documentation]
//
// [ARM CMSIS-Pack documentation]: https://arm-software.github.io/CMSIS_5/Pack/html/algorithmFunc.html

use ufmt::uWrite;
use ufmt::uwrite;

const FLASH_SECTOR_SIZE: u32 = 4096;
// const FLASH_BLOCK_SIZE: u32 = 65536;

pub struct Uart;

impl uWrite for Uart {
    type Error = ();
    fn write_str(&mut self, s: &str) -> Result<(), ()> {
        Ok(for &b in s.as_bytes() {
            unsafe { (rom_funcs().uart_tx_one_char)(b) };
        })
    }
}

// #[allow(unused)]
// extern "C" {
//     fn esp_rom_spiflash_wait_idle(/* esp_rom_spiflash_chip_t *spi */);

//     // fn esp_rom_spiflash_write_encrypted_enable();
//     // fn esp_rom_spiflash_write_encrypted_disable();
//     // fn esp_rom_spiflash_write_encrypted(addr: u32, data: *const u8, len: u32);
    
//     fn esp_rom_spiflash_erase_chip() -> i32;
//     fn esp_rom_spiflash_erase_block(block_number: u32) -> i32;
//     // fn esp_rom_spiflash_erase_sector(sector_number: u32) -> i32;

//     /// address (4 byte alignment), data, length
//     fn esp_rom_spiflash_write(dest_addr: u32, data: *const u32, len: u32) -> i32;
//     /// address (4 byte alignment), data, length
//     fn esp_rom_spiflash_read(src_addr: u32, data: *const u32, len: u32) -> i32;

//     // fn esp_rom_spiflash_unlock() -> i32;
//     // fn esp_rom_spiflash_lock(); // can't find in idf defs?

//     // fn esp_rom_spiflash_config_param();
//     // fn esp_rom_spiflash_read_user_cmd();
//     // fn esp_rom_spiflash_select_qio_pins();
//     // fn esp_rom_spi_flash_auto_sus_res();
//     // fn esp_rom_spi_flash_send_resume();
//     // fn esp_rom_spi_flash_update_id();
//     // fn esp_rom_spiflash_config_clk();
//     // fn esp_rom_spiflash_config_readmode();

//     fn esp_rom_spiflash_read_status(/* esp_rom_spiflash_chip_t *spi ,*/ status: *mut u32);
//     fn esp_rom_spiflash_read_statushigh(/* esp_rom_spiflash_chip_t *spi ,*/ status: *mut u32);
//     fn esp_rom_spiflash_write_status(/* esp_rom_spiflash_chip_t *spi ,*/ status: *mut u32);
//     fn esp_rom_spiflash_attach(config: u32, legacy: bool);



// }

struct RomTable {
    uart_tx_one_char: unsafe extern "C" fn(u8) -> i32,
    ets_efuse_get_spiconfig: unsafe extern "C" fn() -> u32,
    esp_rom_spiflash_attach: unsafe extern "C" fn(u32, bool),
    esp_rom_spiflash_unlock: unsafe extern "C" fn() -> i32,
    esp_rom_spiflash_erase_sector: unsafe extern "C" fn(u32) -> i32,
    esp_rom_spiflash_write: unsafe extern "C" fn(u32, *const u8, u32) -> i32,
}

// TODO if probe-rs supports loading at a fix position, we wont need to generate PIC, and we can go back to using `PROVIDE` in the linker script
fn rom_funcs() -> RomTable {
    unsafe {
        RomTable {
            uart_tx_one_char: addr2func(0x40000068),
            ets_efuse_get_spiconfig: addr2func(0x4000071c),

            esp_rom_spiflash_attach: addr2func(0x40000164),
            esp_rom_spiflash_unlock: addr2func(0x40000140),
            esp_rom_spiflash_erase_sector: addr2func(0x40000128),
            esp_rom_spiflash_write: addr2func(0x4000012c),
        }
    }
}

unsafe fn addr2func<T>(adr: usize) -> T {
    core::mem::transmute_copy(&adr)
}

#[cfg(feature = "standalone")]
#[riscv_rt::entry]
fn main() -> ! {
    let mut _tmp: u32;
    unsafe { asm!("csrrsi {0}, mstatus, {1}", out(reg) _tmp, const 0x00000008) };
    disable_wdts();

    Uart.write_str("MAIN\n").ok();

    let init_res = unsafe { Init(0, 0, 0) };
    // write!(Uart, "RES: {}", init_res).unwrap();

    loop {}
}

/// Setup the device for the
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn Init(_adr: u32, _clk: u32, _fnc: u32) -> i32 {
    Uart.write_str("INIT\n").ok();

    let r = rom_funcs();

    let spiconfig: u32 = (r.ets_efuse_get_spiconfig)();

    (r.esp_rom_spiflash_attach)(spiconfig, false);

    let res = (r.esp_rom_spiflash_unlock)();
    if res != 0 {
        return res;
    }

    0
}

/// Erase the sector at the given address in flash
///
/// Returns 0 on success, 1 on failure.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn EraseSector(adr: u32) -> i32 {
    let r = rom_funcs();

    let res = (r.esp_rom_spiflash_unlock)();
    if res != 0 {
        return 4;
    }

    uwrite!(Uart, "ERASE @ {} - block: {}", adr, (adr - FlashDevice.dev_addr) / FLASH_SECTOR_SIZE).ok();

    let res = (r.esp_rom_spiflash_erase_sector)((adr - FlashDevice.dev_addr) / FLASH_SECTOR_SIZE);
    if res != 0 {
        return 5;
    }
    
    0
}

#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn EraseChip() -> i32 {
    // esp_rom_spiflash_erase_chip()
    0
}

#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ProgramPage(adr: u32, sz: u32, buf: *const u8) -> i32 {
    let r = rom_funcs();

    let res = (r.esp_rom_spiflash_unlock)();
    if res != 0 {
        return res;
    }

    let buf_addr: u32 = buf as *const _ as _;
    if buf_addr % 4 != 0 { // TODO write into aligned buffer first, then pass to write if unaligned
        return 12;
    }

    let res = (r.esp_rom_spiflash_write)(adr - FlashDevice.dev_addr, buf, sz);
    if res != 0 {
        return 7;
    }

    0
}

#[no_mangle]
#[inline(never)]
pub extern "C" fn UnInit(_fnc: u32) -> i32 {
    0 // TODO - what needs to be uninitialized?
}

#[allow(non_upper_case_globals)]
#[no_mangle]
#[used]
#[link_section = "DeviceData"]
pub static FlashDevice: FlashDeviceDescription = FlashDeviceDescription {
    vers: 0x0001,
    dev_name: [0u8; 128],
    dev_type: 5,
    dev_addr: 0x4200_0000,
    device_size: 0x1_000_000, /* this is variable? - set to max of 16MB */
    page_size: 256,
    _reserved: 0,
    empty: 0xFF,
    program_time_out: 1000,
    erase_time_out: 2000,
    flash_sectors: sectors(),
};

const fn sectors() -> [FlashSector; 512] {
    let mut sectors = [FlashSector::default(); 512];

    sectors[0] = FlashSector {
        size: 0x1000,
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

const SECTOR_END: FlashSector = FlashSector {
    size: 0xffff_ffff,
    address: 0xffff_ffff,
};


use core::panic::PanicInfo;
#[panic_handler]
fn panic(_: &PanicInfo<'_>) -> ! {
    loop {}
}


pub fn disable_wdts() {
    unsafe {
        // super wdt
        core::ptr::write_volatile(0x600080B0 as *mut _, 0x8F1D312Au32); // disable write protect
        core::ptr::write_volatile(
            0x600080AC as *mut _,
            core::ptr::read_volatile(0x600080AC as *const u32) | 1 << 31,
        ); // set RTC_CNTL_SWD_AUTO_FEED_EN
        core::ptr::write_volatile(0x600080B0 as *mut _, 0u32); // enable write protect

        // tg0 wdg
        core::ptr::write_volatile(0x6001f064 as *mut _, 0x50D83AA1u32); // disable write protect
        core::ptr::write_volatile(0x6001F048 as *mut _, 0u32);
        core::ptr::write_volatile(0x6001f064 as *mut _, 0u32); // enable write protect

        // tg1 wdg
        core::ptr::write_volatile(0x60020064 as *mut _, 0x50D83AA1u32); // disable write protect
        core::ptr::write_volatile(0x60020048 as *mut _, 0u32);
        core::ptr::write_volatile(0x60020064 as *mut _, 0u32); // enable write protect

        // rtc wdg
        core::ptr::write_volatile(0x600080a8 as *mut _, 0x50D83AA1u32); // disable write protect
        core::ptr::write_volatile(0x60008090 as *mut _, 0u32);
        core::ptr::write_volatile(0x600080a8 as *mut _, 0u32); // enable write protect
    }
}