#![no_std]
#![no_main]

// Define necessary functions for flash loader
//
// These are taken from the [ARM CMSIS-Pack documentation]
//
// [ARM CMSIS-Pack documentation]: https://arm-software.github.io/CMSIS_5/Pack/html/algorithmFunc.html

// use panic_never as _;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

const FLASH_SECTOR_SIZE: u32 = 4096;

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
    fn esp_rom_spiflash_wait_idle(/* esp_rom_spiflash_chip_t *spi */);

    // fn esp_rom_spiflash_write_encrypted_enable();
    // fn esp_rom_spiflash_write_encrypted_disable();
    // fn esp_rom_spiflash_write_encrypted(addr: u32, data: *const u8, len: u32);

    fn esp_rom_spiflash_erase_chip() -> i32;
    fn esp_rom_spiflash_erase_block(block_number: u32) -> i32;
    fn esp_rom_spiflash_erase_sector(sector_number: u32) -> i32;

    /// address (4 byte alignment), data, length
    fn esp_rom_spiflash_write(dest_addr: u32, data: *const u8, len: u32) -> i32;
    /// address (4 byte alignment), data, length
    fn esp_rom_spiflash_read(src_addr: u32, data: *const u32, len: u32) -> i32;

    fn esp_rom_spiflash_unlock() -> i32;
    // fn esp_rom_spiflash_lock(); // can't find in idf defs?

    // fn esp_rom_spiflash_config_param();
    // fn esp_rom_spiflash_read_user_cmd();
    // fn esp_rom_spiflash_select_qio_pins();
    // fn esp_rom_spi_flash_auto_sus_res();
    // fn esp_rom_spi_flash_send_resume();
    // fn esp_rom_spi_flash_update_id();
    // fn esp_rom_spiflash_config_clk();
    // fn esp_rom_spiflash_config_readmode();

    // fn esp_rom_spiflash_read_status(/* esp_rom_spiflash_chip_t *spi ,*/ status: *mut u32);
    // fn esp_rom_spiflash_read_statushigh(/* esp_rom_spiflash_chip_t *spi ,*/ status: *mut u32);
    // fn esp_rom_spiflash_write_status(/* esp_rom_spiflash_chip_t *spi ,*/ status: *mut u32);
    fn esp_rom_spiflash_attach(config: u32, legacy: bool);
    fn esp_rom_spiflash_config_clk(div: u8, spi: u8) -> i32;

    fn uart_tx_one_char(byte: u8);

    fn ets_efuse_get_spiconfig() -> u32;
    fn ets_get_apb_freq() -> u32;

}

/// Setup the device for the
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn Init(_adr: u32, _clk: u32, _fnc: u32) -> i32 {
    static mut INITD: bool = false;

    if !INITD {
        // TODODODOODODODODOD
        // TODO higher speed apb doesn't seem to affectflash speed, probably needs qio or something - look into how esptool flashes so fast
        //    

        dprintln!("INIT - APB freq: {}", ets_get_apb_freq());

        // todo setup higher speed clocks
        let peripherals = esp32c3::Peripherals::steal();

        let system = peripherals.SYSTEM;

        let x = system.sysclk_conf.read();

        dprintln!("DIVIDER: {}", x.pre_div_cnt().bits());
        dprintln!("CLOCK_SEL: {}", x.soc_clk_sel().bits());
        dprintln!("XTAL_FREQ: {}", x.clk_xtal_freq().bits());

        system.sysclk_conf.modify(|_, w| {
            w.pre_div_cnt().bits(0b0) // 40mhz
        });

        let res = esp_rom_spiflash_config_clk(1, 0);
        if res != 0 {
            return res;
        }

        let spiconfig: u32 = ets_efuse_get_spiconfig();
        // let spiconfig = 1; // hspi

        esp_rom_spiflash_attach(spiconfig, false);

        let res = esp_rom_spiflash_unlock();
        if res != 0 {
            return res;
        }

        dprintln!("DIVIDER: {}", x.pre_div_cnt().bits());
        dprintln!("CLOCK_SEL: {}", x.soc_clk_sel().bits());
        dprintln!("XTAL_FREQ: {}", x.clk_xtal_freq().bits());

        dprintln!("INIT - APB freq after: {}", ets_get_apb_freq());

        INITD = true;
    }

    0
}

/// Erase the sector at the given address in flash
///
/// Returns 0 on success, 1 on failure.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn EraseSector(adr: u32) -> i32 {
    let res = esp_rom_spiflash_unlock();
    if res != 0 {
        return res;
    }

    dprintln!("ERASE @ {}", adr);
    let res = esp_rom_spiflash_erase_sector(adr / FLASH_SECTOR_SIZE);
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
    let res = esp_rom_spiflash_unlock();
    if res != 0 {
        return res;
    }

    let buf_addr: u32 = buf as *const _ as _;
    if buf_addr % 4 != 0 {
        return res;
    }

    dprintln!("PROGRAM {} bytes @ {}", sz, adr);

    let res = esp_rom_spiflash_write(adr, buf, sz);
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
