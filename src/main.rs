#![no_std]
#![no_main]

// Define necessary functions for flash loader
//
// These are taken from the [ARM CMSIS-Pack documentation]
//
// [ARM CMSIS-Pack documentation]: https://arm-software.github.io/CMSIS_5/Pack/html/algorithmFunc.html

use panic_never as _;


const FLASH_SECTOR_SIZE: u32 = 4096;

#[cfg(feature = "log")]
mod log {

    use ufmt::uWrite;
    use crate::rom_funcs;
    pub struct Uart;

    impl uWrite for Uart {
        type Error = ();
        fn write_str(&mut self, s: &str) -> Result<(), ()> {
            Ok(for &b in s.as_bytes() {
                unsafe { (rom_funcs().uart_tx_one_char)(b) };
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
        () => {
            
        };
        ($fmt:expr) => {
            
        };
        ($fmt:expr, $($arg:tt)*) => {
            
        };
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

    // cache_owner_init: unsafe extern "C" fn (),
    // cache_mmu_init: unsafe extern "C" fn (),
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

            // cache_owner_init: addr2func(0x40000554),
        }
    }
}

unsafe fn addr2func<T>(adr: usize) -> T {
    core::mem::transmute_copy(&adr)
}

/// Setup the device for the
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn Init(_adr: u32, _clk: u32, _fnc: u32) -> i32 {
    dprintln!("INIT");
    // todo setup higher speed clocks

    let r = rom_funcs();

    let spiconfig: u32 = (r.ets_efuse_get_spiconfig)();
    // let spiconfig = 1; // hspi

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
        return res;
    }

    dprintln!("ERASE @ {}", adr);
    let res = (r.esp_rom_spiflash_erase_sector)(adr / FLASH_SECTOR_SIZE);
    if res != 0 {
        return res;
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
    if buf_addr % 4 != 0 {
        return res;
    }

    dprintln!("PROGRAM {} bytes @ {}",  sz, adr);
        
    let res = (r.esp_rom_spiflash_write)(adr, buf, sz);
    if res != 0 {
        return res;
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
    dev_addr: 0x0,
    device_size: 0x1_000_000, /* this is variable? - set to max of 16MB */
    page_size: 4096,
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
