#![no_std]
#![no_main]

// Define necessary functions for flash loader
//
// These are taken from the [ARM CMSIS-Pack documentation]
//
// [ARM CMSIS-Pack documentation]: https://arm-software.github.io/CMSIS_5/Pack/html/algorithmFunc.html

use core::mem::MaybeUninit;

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
    fn esp_rom_spiflash_read(src_addr: u32, data: *const u32, len: u32) -> i32;
    fn esp_rom_spiflash_read_user_cmd(status: *mut u32, cmd: u8) -> i32;
    fn esp_rom_spiflash_unlock() -> i32;
    // fn esp_rom_spiflash_lock(); // can't find in idf defs?
    fn esp_rom_spiflash_attach(config: u32, legacy: bool);

    fn uart_tx_one_char(byte: u8);

    fn ets_efuse_get_spiconfig() -> u32;

    /// Main low-level decompressor coroutine function. This is the only function actually needed
    /// for decompression. All the other functions are just high-level helpers for improved
    /// usability.
    /// This is a universal API, i.e. it can be used as a building block to build any desired higher
    /// level decompression API. In the limit case, it can be called once per every byte input or
    /// output.
    fn tinfl_decompress(
        decompressor: *mut TinflDecompressor,
        next_in: *const u8,
        in_bytes: *mut usize,
        out_buf_start: *mut u8,
        next_out: *mut u8,
        out_bytes: *mut usize,
        flags: u32,
    ) -> TinflStatus;
}

type TinflStatus = i8;
// const TINFL_STATUS_FAILED_CANNOT_MAKE_PROGRESS: TinflStatus = -4;
// const TINFL_STATUS_BAD_PARAM: TinflStatus = -3;
// const TINFL_STATUS_ADLER32_MISMATCH: TinflStatus = -2;
// const TINFL_STATUS_FAILED: TinflStatus = -1;
const TINFL_STATUS_DONE: TinflStatus = 0;
const TINFL_STATUS_NEEDS_MORE_INPUT: TinflStatus = 1;
// const TINFL_STATUS_HAS_MORE_OUTPUT: TinflStatus = 2;

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

const TINFL_MAX_HUFF_SYMBOLS_0: usize = 288;
const TINFL_MAX_HUFF_TABLES: usize = 3;
const TINFL_MAX_HUFF_SYMBOLS_1: usize = 32;
const TINFL_FAST_LOOKUP_BITS: usize = 10;
const TINFL_FAST_LOOKUP_SIZE: usize = 1 << TINFL_FAST_LOOKUP_BITS;

// Decompression flags used by tinfl_decompress().

/// If set, the input has a valid zlib header and ends with an adler32 checksum (it's a valid zlib
/// stream). Otherwise, the input is a raw deflate stream.
const TINFL_FLAG_PARSE_ZLIB_HEADER: u32 = 1;

/// If set, there are more input bytes available beyond the end of the supplied input buffer.
/// If clear, the input buffer contains all remaining input.
const TINFL_FLAG_HAS_MORE_INPUT: u32 = 2;

/// If set, the output buffer is large enough to hold the entire decompressed stream.
/// If clear, the output buffer is at least the size of the dictionary (typically 32KB).
// const TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF: u32 = 4;

/// Force adler-32 checksum computation of the decompressed bytes.
// const TINFL_FLAG_COMPUTE_ADLER32: u32 = 8;

const ERROR_BASE_INTERNAL: i32 = -1000;
const ERROR_BASE_TINFL: i32 = -2000;
const ERROR_BASE_FLASH: i32 = -4000;

#[repr(C)]
struct TinflHuffTable {
    m_code_size: [u8; TINFL_MAX_HUFF_SYMBOLS_0],
    m_look_up: [u16; TINFL_FAST_LOOKUP_SIZE],
    m_tree: [u16; TINFL_MAX_HUFF_SYMBOLS_0 * 2],
}

#[repr(C)]
struct TinflDecompressor {
    m_state: u32,
    m_num_bits: u32,
    m_zhdr0: u32,
    m_zhdr1: u32,
    m_z_adler32: u32,
    m_final: u32,
    m_type: u32,
    m_check_adler32: u32,
    m_dist: u32,
    m_counter: u32,
    m_num_extra: u32,
    m_table_sizes: [u32; TINFL_MAX_HUFF_TABLES],
    m_bit_buf: u32,
    m_dist_from_out_buf_start: usize,
    m_tables: [TinflHuffTable; TINFL_MAX_HUFF_TABLES],
    m_raw_header: [u8; 4],
    m_len_codes: [u8; TINFL_MAX_HUFF_SYMBOLS_0 + TINFL_MAX_HUFF_SYMBOLS_1 + 137],
}

struct OutBuffer {
    buffer: [u8; 32768],
    len: usize,
}

impl OutBuffer {
    fn space(&self) -> usize {
        self.buffer.len() - self.len
    }

    unsafe fn pointers(&mut self) -> (*mut u8, *mut u8) {
        let write_idx = self.len;
        let start = core::ptr::addr_of_mut!(self.buffer).cast();

        (start, start.add(write_idx))
    }

    fn full(&self) -> bool {
        self.space() == 0
    }

    fn take<R>(&mut self, out: impl FnOnce(&[u8]) -> R) -> R {
        let data = unsafe {
            // self.len is always <= self.buffer.len()
            let len = core::mem::take(&mut self.len);
            self.buffer.get_unchecked(..len)
        };

        out(data)
    }
}

impl TinflDecompressor {
    fn decompress(&mut self, input: &mut &[u8], out: &mut OutBuffer, last: bool) -> i32 {
        let flags = if last {
            TINFL_FLAG_PARSE_ZLIB_HEADER
        } else {
            TINFL_FLAG_PARSE_ZLIB_HEADER | TINFL_FLAG_HAS_MORE_INPUT
        };

        let mut in_bytes = input.len();
        let data_buf = input.as_ptr();

        let mut out_bytes = out.space();
        let (out_buf, next_out) = unsafe { out.pointers() };

        let status = unsafe {
            tinfl_decompress(
                self,
                data_buf,
                &mut in_bytes,
                out_buf,
                next_out,
                &mut out_bytes,
                flags,
            )
        } as i32;

        if in_bytes > input.len() {
            // tinfl_decompress() shouldn't have consumed more bytes than we gave it
            // but let's not trust it
            return ERROR_BASE_INTERNAL - 1;
        }

        // Consume processed input
        *input = &input[in_bytes..];

        // Update output buffer
        out.len += out_bytes;

        status
    }
}

struct Decompressor {
    decompressor: TinflDecompressor,
    output: OutBuffer,
    image_start: u32,
    offset: u32,
    remaining_compressed: usize,
}

impl Decompressor {
    unsafe fn get<'a>() -> &'a mut Self {
        &mut *Self::get_ptr()
    }

    unsafe fn get_ptr() -> *mut Self {
        static mut DECOMPRESSOR: MaybeUninit<Decompressor> = MaybeUninit::uninit();

        DECOMPRESSOR.as_mut_ptr().cast()
    }

    unsafe fn init() {
        let this = Self::get_ptr();
        Self::init_impl(this, 0xFFFF_FFFF, 0);
    }

    unsafe fn init_impl(this: *mut Self, address: u32, compressed: u32) {
        (*this).image_start = address;
        (*this).offset = 0;
        (*this).decompressor.m_state = 0;
        (*this).output.len = 0;
        (*this).remaining_compressed = compressed as usize;
    }

    fn reinit(&mut self, address: u32, compressed: u32) {
        unsafe { Self::init_impl(self, address, compressed) }
    }

    fn decompress(&mut self, input: &[u8]) -> i32 {
        if self.remaining_compressed == 0 {
            return ERROR_BASE_INTERNAL - 2;
        }

        // We may have to cut off some padding bytes.
        let chunk_len = self.remaining_compressed.min(input.len());
        self.remaining_compressed -= chunk_len;

        // Signal tinfl_decompress() that this is the last chunk
        let last = self.remaining_compressed == 0;

        // Iterate through all the input
        let mut input = &input[..chunk_len];
        let mut status = TINFL_STATUS_NEEDS_MORE_INPUT as i32;
        while !input.is_empty() && status > TINFL_STATUS_DONE as i32 {
            status = self
                .decompressor
                .decompress(&mut input, &mut self.output, last);

            if status == TINFL_STATUS_DONE as i32 || self.output.full() {
                // We're either finished or the decompressor can't continue
                // until we flush the buffer.
                let flush_status = self.flush();

                if flush_status < 0 {
                    return ERROR_BASE_FLASH + flush_status;
                }
            }
        }

        if status < TINFL_STATUS_DONE as i32 {
            ERROR_BASE_TINFL + status as i32
        } else {
            0
        }
    }

    fn flush(&mut self) -> i32 {
        let mut offset = self.offset;
        let address = self.image_start + offset;

        // Take buffer contents, write to flash and update offset.
        let status = self.output.take(|data| {
            offset += data.len() as u32;
            write_flash(address, data)
        });

        self.offset = offset;

        status
    }

    fn read_compressed_length(data: &mut &[u8]) -> Result<u32, i32> {
        if data.len() < 4 {
            // We don't have enough bytes to read the length
            return Err(ERROR_BASE_INTERNAL - 3);
        }

        // Image length is prepended to the first chunk, cut it off.
        let (length_bytes, remaining) = data.split_at(4);
        *data = remaining;

        Ok(u32::from_le_bytes([
            length_bytes[0],
            length_bytes[1],
            length_bytes[2],
            length_bytes[3],
        ]))
    }
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

        // TODO: raise CPU frequency

        esp_rom_spiflash_attach(spiconfig, false);

        Decompressor::init();

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
    dprintln!("ERASE @ {}", adr);
    esp_rom_spiflash_erase_block(adr / FLASH_BLOCK_SIZE)
}

#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn EraseChip() -> i32 {
    esp_rom_spiflash_erase_chip()
}

fn write_flash(address: u32, data: &[u8]) -> i32 {
    if data.is_empty() {
        return 0;
    }
    let len = data.len() as u32;
    unsafe { esp_rom_spiflash_write(address, data.as_ptr(), len) }
}

fn program(decompressor: &mut Decompressor, address: u32, mut data: &[u8]) -> i32 {
    if address != decompressor.image_start {
        // Finish previous image
        decompressor.flush();

        match Decompressor::read_compressed_length(&mut data) {
            // Initialize decompressor with new image.
            Ok(compressed_length) => decompressor.reinit(address, compressed_length),
            Err(e) => return e,
        }
    }

    decompressor.decompress(data)
}

#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ProgramPage(adr: u32, sz: u32, buf: *const u8) -> i32 {
    if (buf as u32) % 4 != 0 {
        dprintln!("ERROR buf not word aligned");
        return ERROR_BASE_INTERNAL - 0xAA;
    }

    dprintln!("PROGRAM {} bytes @ {}", sz, adr);
    let input = core::slice::from_raw_parts(buf, sz as usize);

    let decompressor = Decompressor::get();
    program(decompressor, adr, input)
}

#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn UnInit(_fnc: u32) -> i32 {
    let decompressor = Decompressor::get();
    decompressor.flush();

    // The flash ROM functions don't wait for the end of the last operation.
    wait_for_idle()
}

// esptool uses 16k for the buffer
const PAGE_SIZE: u32 = 0x4000;

#[allow(non_upper_case_globals)]
#[no_mangle]
#[used]
#[link_section = "DeviceData"]
pub static FlashDevice: FlashDeviceDescription = FlashDeviceDescription {
    vers: 0x0001,
    dev_name: [0u8; 128],
    dev_type: 5,
    dev_addr: 0x0,
    device_size: 0x1000000, // Max of 16MB
    page_size: PAGE_SIZE,
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
