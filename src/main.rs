#![no_std]
#![no_main]
#![cfg_attr(
    target_arch = "xtensa",
    feature(asm_experimental_arch, naked_functions)
)]

// Target memory configuration

// Decompressor is 43776 bytes, reserve more in case compiler changes layout
const _: [u8; 43776] = [0; core::mem::size_of::<Decompressor>()];

// Placement:
// - Xtensa: Pin stack top first, calculate backwards:
//  - 32K stack
//  - 32K for data pages
//  - 64K for decompressor state
// - RISC-V: At the end of memory, calculate backwards:
//  - 64K for data pages (32K needed, but 64K is easier to calculate)
//  - 64K for decompressor state
//  - stack comes automatically after the loader

// Xtensa   | Image IRAM  | Image DRAM  | STATE_ADDR  | data_load_addr | Stack (top)
// -------- | ----------- | ----------- | ----------- | -------------- | -----------
// ESP32    | 0x4009_0000 | -           | 0x3FFC_0000 | 0x3FFD_0000    | 0x3FFE_0000
// ESP32-S2 | 0x4002_C400 | 0x3FFB_C400 | 0x3FFB_E000 | 0x3FFC_E000    | 0x3FFD_F000
// ESP32-S3 | 0x4038_0400 | 0x3FC9_0400 | 0x3FCB_0000 | 0x3FCC_0000    | 0x3FCD_0000

// RISC-V   | Image IRAM  | Image DRAM  | STATE_ADDR  | data_load_addr | DRAM end (avoiding cache)
// -------- | ----------- | ----------- | ----------- | -------------- | -----------
// ESP32-C2 | 0x4038_C000 | 0x3FCA_C000 | 0x3FCB_0000 | 0x3FCC_0000    | 0x3FCD_0000
// ESP32-C3 | 0x4039_0000 | 0x3FC1_0000 | 0x3FCB_0000 | 0x3FCC_0000    | 0x3FCD_0000
// ESP32-C6 | 0x4081_0000 | 0x4081_0000 | 0x4084_0000 | 0x4085_0000    | 0x4086_0000
// ESP32-H2 | 0x4081_0000 | 0x4081_0000 | 0x4082_0000 | 0x4083_0000    | 0x4083_8000 !! has smaller RAM, only reserve 32K for data

// "State" base address
#[cfg(feature = "esp32")]
const STATE_ADDR: usize = 0x3FFC_0000;
#[cfg(feature = "esp32s2")]
const STATE_ADDR: usize = 0x3FFB_E000;
#[cfg(feature = "esp32s3")]
const STATE_ADDR: usize = 0x3FCB_0000;
#[cfg(feature = "esp32c2")]
const STATE_ADDR: usize = 0x3FCB_0000;
#[cfg(feature = "esp32c3")]
const STATE_ADDR: usize = 0x3FCB_0000;
#[cfg(feature = "esp32c6")]
const STATE_ADDR: usize = 0x4086_0000;
#[cfg(feature = "esp32h2")]
const STATE_ADDR: usize = 0x4082_0000;

// End of target memory configuration

// Define necessary functions for flash loader
//
// These are taken from the [ARM CMSIS-Pack documentation]
//
// [ARM CMSIS-Pack documentation]: https://arm-software.github.io/CMSIS_5/Pack/html/algorithmFunc.html

use panic_never as _;

use crate::tinfl::{
    OutBuffer, TinflDecompressor, TINFL_STATUS_DONE, TINFL_STATUS_NEEDS_MORE_INPUT,
};

#[cfg_attr(any(target_arch = "xtensa"), path = "api_xtensa.rs")]
mod api;
mod flash;
mod properties;
mod tinfl;

#[cfg(not(any(target_arch = "xtensa", target_arch = "riscv32")))]
compile_error!("specify the target with `--target`");

const ERROR_BASE_INTERNAL: i32 = -1000;
const ERROR_BASE_TINFL: i32 = -2000;
const ERROR_BASE_FLASH: i32 = -4000;

#[cfg(feature = "log")]
mod log {

    extern "C" {
        fn uart_tx_one_char(byte: u8);
    }

    use ufmt::uWrite;
    pub struct Uart;

    impl uWrite for Uart {
        type Error = ();
        fn write_str(&mut self, s: &str) -> Result<(), ()> {
            for &b in s.as_bytes() {
                unsafe { uart_tx_one_char(b) };
            }
            Ok(())
        }
    }
}

#[cfg(feature = "log")]
#[macro_export]
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

const INITED_MAGIC: u32 = 0xAAC0FFEE;
const INITED: *mut u32 = STATE_ADDR as *mut u32;
const DECOMPRESSOR: *mut Decompressor = (STATE_ADDR + 4) as *mut Decompressor;

fn is_inited() -> bool {
    unsafe { *INITED == INITED_MAGIC }
}

/// Setup the device for the flashing process.
#[no_mangle]
pub unsafe extern "C" fn Init_impl(_adr: u32, _clk: u32, _fnc: u32) -> i32 {
    dprintln!("INIT");

    flash::attach();

    *DECOMPRESSOR = Decompressor::new();
    *INITED = INITED_MAGIC;

    0
}

/// Erase the sector at the given address in flash
///
/// Returns 0 on success, 1 on failure.
#[no_mangle]
pub unsafe extern "C" fn EraseSector_impl(adr: u32) -> i32 {
    if !is_inited() {
        return ERROR_BASE_INTERNAL - 1;
    };
    flash::erase_block(adr)
}

#[no_mangle]
pub unsafe extern "C" fn EraseChip_impl() -> i32 {
    if !is_inited() {
        return ERROR_BASE_INTERNAL - 1;
    };
    flash::erase_chip()
}

#[no_mangle]
pub unsafe extern "C" fn ProgramPage_impl(adr: u32, sz: u32, buf: *const u8) -> i32 {
    if !is_inited() {
        return ERROR_BASE_INTERNAL - 1;
    };

    if (buf as u32) % 4 != 0 {
        dprintln!("ERROR buf not word aligned");
        return ERROR_BASE_INTERNAL - 5;
    }

    dprintln!("PROGRAM {} bytes @ {}", sz, adr);

    let input = core::slice::from_raw_parts(buf, sz as usize);

    (*DECOMPRESSOR).program(adr, input)
}

#[no_mangle]
pub unsafe extern "C" fn ReadFlash_impl(adr: u32, sz: u32, buf: *mut u8) -> i32 {
    if !is_inited() {
        return ERROR_BASE_INTERNAL - 1;
    };

    if (buf as u32) % 4 != 0 {
        dprintln!("ERROR buf not word aligned");
        return ERROR_BASE_INTERNAL - 5;
    }

    dprintln!("READ FLASH {} bytes @ {}", sz, adr);

    let buffer = core::slice::from_raw_parts_mut(buf, sz as usize);
    crate::flash::read_flash(adr, buffer)
}

#[no_mangle]
pub unsafe extern "C" fn UnInit_impl(_fnc: u32) -> i32 {
    if !is_inited() {
        return ERROR_BASE_INTERNAL - 1;
    };

    (*DECOMPRESSOR).flush();

    // The flash ROM functions don't wait for the end of the last operation.
    let r = flash::wait_for_idle();

    *INITED = 0;

    r
}

pub struct Decompressor {
    decompressor: TinflDecompressor,
    output: OutBuffer,
    image_start: u32,
    offset: u32,
    remaining_compressed: usize,
}

impl Decompressor {
    pub fn new() -> Self {
        Self {
            image_start: 0xFFFF_FFFF,
            offset: 0,
            output: OutBuffer::new(),
            remaining_compressed: 0,
            decompressor: TinflDecompressor::new(),
        }
    }

    fn reinit(&mut self, address: u32, compressed: u32) {
        self.image_start = address;
        self.offset = 0;

        self.remaining_compressed = compressed as usize;

        self.decompressor = TinflDecompressor::new();
        self.output.take(|_| {});
    }

    fn decompress(&mut self, input: &[u8]) -> i32 {
        if self.remaining_compressed == 0 {
            return ERROR_BASE_INTERNAL - 3;
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
            ERROR_BASE_TINFL + status
        } else {
            0
        }
    }

    pub fn flush(&mut self) -> i32 {
        let mut offset = self.offset;
        let address = self.image_start + offset;

        // Take buffer contents, write to flash and update offset.
        let status = self.output.take(|data| {
            offset += data.len() as u32;
            crate::flash::write_flash(address, data)
        });

        self.offset = offset;

        status
    }

    pub fn program(&mut self, address: u32, mut data: &[u8]) -> i32 {
        if self.image_start != address {
            // Finish previous image
            self.flush();

            if data.len() < 4 {
                // We don't have enough bytes to read the length
                return ERROR_BASE_INTERNAL - 4;
            }

            // Image length is prepended to the first chunk, cut it off.
            let (length_bytes, remaining) = data.split_at(4);
            data = remaining;

            let compressed_length = u32::from_le_bytes([
                length_bytes[0],
                length_bytes[1],
                length_bytes[2],
                length_bytes[3],
            ]);

            self.reinit(address, compressed_length);
        }
        self.decompress(data)
    }
}
