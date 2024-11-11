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
// ESP32-P4 | 0x4FF1_0000 | 0x4FF1_0000 | 0x4FFA_0000 | 0x4FFB_0000    | 0x4FFC_0000

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
#[cfg(feature = "esp32p4")]
const STATE_ADDR: usize = 0x4FFA_0000;


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
pub unsafe extern "C" fn Verify_impl(adr: u32, sz: u32, buf: *const u8) -> i32 {
    if !is_inited() {
        return ERROR_BASE_INTERNAL - 1;
    };

    if (buf as u32) % 4 != 0 {
        dprintln!("ERROR buf not word aligned");
        return ERROR_BASE_INTERNAL - 5;
    }

    dprintln!("PROGRAM {} bytes @ {}", sz, adr);

    let input = core::slice::from_raw_parts(buf, sz as usize);

    (*DECOMPRESSOR).verify(adr, input)
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

    let buf = core::slice::from_raw_parts_mut(buf, sz as usize);
    crate::flash::read_flash(adr, buf)
}

#[no_mangle]
pub unsafe extern "C" fn UnInit_impl(fnc: u32) -> i32 {
    if !is_inited() {
        return ERROR_BASE_INTERNAL - 1;
    };

    *INITED = 0;

    if fnc == 2 {
        // The flash ROM functions don't wait for the end of the last operation.
        flash::wait_for_idle()
    } else {
        0
    }
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

    fn decompress(&mut self, input: &[u8], process: fn(u32, &[u8]) -> i32) -> i32 {
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
                let flush_status = self.flush(process);

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

    pub fn flush(&mut self, process: fn(address: u32, data: &[u8]) -> i32) -> i32 {
        let mut offset = self.offset;
        let address = self.image_start + offset;

        // Take buffer contents, write to flash and update offset.
        let status = self.output.take(|data| {
            offset += data.len() as u32;

            process(address, data)
        });

        self.offset = offset;

        status
    }

    fn handle_compressed(
        &mut self,
        address: u32,
        mut data: &[u8],
        process: fn(u32, &[u8]) -> i32,
    ) -> i32 {
        if self.image_start != address {
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
        self.decompress(data, process)
    }

    pub fn program(&mut self, address: u32, data: &[u8]) -> i32 {
        self.handle_compressed(address, data, write_to_flash)
    }

    pub fn verify(&mut self, address: u32, data: &[u8]) -> i32 {
        // We're supposed to return the address up to which we've verified.
        // However, we process compressed data and the caller expects us to respond in terms of
        // compressed offsets, so we don't actually know where comparison fails.
        let status = if self.handle_compressed(address, data, verify_flash) == 0 {
            address + data.len() as u32
        } else {
            address
        };

        status as i32
    }
}

fn write_to_flash(address: u32, data: &[u8]) -> i32 {
    let status = crate::flash::write_flash(address, data);

    if status < 0 {
        return ERROR_BASE_FLASH + status;
    }

    0
}

fn verify_flash(mut address: u32, mut data: &[u8]) -> i32 {
    const READBACK_BUFFER: usize = 256;
    let mut readback = unsafe {
        let mut buf = core::mem::MaybeUninit::<[u8; READBACK_BUFFER]>::uninit();
        for i in 0..READBACK_BUFFER {
            buf.as_mut_ptr().cast::<u8>().write_volatile(i as u8);
        }
        buf.assume_init()
    };

    while !data.is_empty() {
        let chunk_size = READBACK_BUFFER.min(data.len());
        let (slice, rest) = unsafe {
            // SAFETY: skip is always at most `data.len()`
            data.split_at_unchecked(chunk_size)
        };
        data = rest;

        let readback_slice = &mut readback[..chunk_size];

        let status = crate::flash::read_flash(address, readback_slice);
        if status < 0 {
            return -1;
        }

        for (a, b) in slice.iter().zip(readback_slice.iter()) {
            if a != b {
                return -1;
            }
        }

        address += chunk_size as u32;
    }

    0
}
