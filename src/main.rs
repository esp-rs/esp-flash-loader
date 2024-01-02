#![no_std]
#![no_main]
#![cfg_attr(
    target_arch = "xtensa",
    feature(asm_experimental_arch, naked_functions)
)]

// Define necessary functions for flash loader
//
// These are taken from the [ARM CMSIS-Pack documentation]
//
// [ARM CMSIS-Pack documentation]: https://arm-software.github.io/CMSIS_5/Pack/html/algorithmFunc.html

use core::ptr::addr_of_mut;

use panic_never as _;

use crate::tinfl::{
    OutBuffer, TinflDecompressor, TINFL_STATUS_DONE, TINFL_STATUS_NEEDS_MORE_INPUT,
};

#[cfg_attr(any(target_arch = "xtensa"), path = "api_xtensa.rs")]
mod api;
mod flash;
mod properties;
mod tinfl;

const ERROR_BASE_INTERNAL: i32 = -1000;
const ERROR_BASE_TINFL: i32 = -2000;
const ERROR_BASE_FLASH: i32 = -4000;

#[cfg(not(any(target_arch = "xtensa", target_arch = "riscv32")))]
compile_error!("specify the target with `--target`");

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

static mut DECOMPRESSOR: Option<Decompressor> = None;

#[cfg(feature = "esp32s2")]
mod chip_specific {
    use core::ops::Range;

    pub const OFFSET: isize = 0x4002_8000 - 0x3FFB_8000;
    pub const IRAM: Range<usize> = 0x4000_0000..0x4007_2000;
}

#[cfg(feature = "esp32s3")]
mod chip_specific {
    use core::ops::Range;

    pub const OFFSET: isize = 0x4037_8000 - 0x3FC8_8000;
    pub const IRAM: Range<usize> = 0x4000_0000..0x403E_0000;
}

// We need to access the page buffers and decompressor on the data bus, otherwise we'll run into
// LoadStoreError exceptions. This should be removed once probe-rs can place data into the correct
// memory region.
fn addr_to_data_bus(addr: usize) -> usize {
    #[cfg(any(feature = "esp32s2", feature = "esp32s3"))]
    {
        if chip_specific::IRAM.contains(&addr) {
            return (addr as isize - chip_specific::OFFSET) as usize;
        }
    }

    addr
}

unsafe fn data_bus<T>(ptr: *const T) -> *const T {
    addr_to_data_bus(ptr as usize) as *const T
}

unsafe fn data_bus_mut<T>(ptr: *mut T) -> *mut T {
    addr_to_data_bus(ptr as usize) as *mut T
}

unsafe fn decompressor<'a>() -> Option<&'a mut Decompressor> {
    let decompressor = addr_of_mut!(DECOMPRESSOR);
    let decompressor = data_bus_mut(decompressor);

    decompressor.as_mut().unwrap_unchecked().as_mut()
}

/// Setup the device for the flashing process.
#[no_mangle]
pub unsafe extern "C" fn Init_impl(_adr: u32, _clk: u32, _fnc: u32) -> i32 {
    if decompressor().is_none() {
        dprintln!("INIT");

        flash::attach();

        DECOMPRESSOR = Some(Decompressor::new());
    }

    0
}

/// Erase the sector at the given address in flash
///
/// Returns 0 on success, 1 on failure.
#[no_mangle]
pub unsafe extern "C" fn EraseSector_impl(adr: u32) -> i32 {
    flash::erase_block(adr)
}

#[no_mangle]
pub unsafe extern "C" fn EraseChip_impl() -> i32 {
    flash::erase_chip()
}

#[no_mangle]
pub unsafe extern "C" fn ProgramPage_impl(adr: u32, sz: u32, buf: *const u8) -> i32 {
    let Some(decompressor) = decompressor() else {
        return ERROR_BASE_INTERNAL - 1;
    };

    if (buf as u32) % 4 != 0 {
        dprintln!("ERROR buf not word aligned");
        return ERROR_BASE_INTERNAL - 5;
    }

    dprintln!("PROGRAM {} bytes @ {}", sz, adr);

    // Access data through the data bus
    let buf = data_bus(buf);

    let input = core::slice::from_raw_parts(buf, sz as usize);

    decompressor.program(adr, input)
}

#[no_mangle]
pub unsafe extern "C" fn UnInit_impl(_fnc: u32) -> i32 {
    let Some(decompressor) = decompressor() else {
        return ERROR_BASE_INTERNAL - 1;
    };

    decompressor.flush();

    // The flash ROM functions don't wait for the end of the last operation.
    flash::wait_for_idle()
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
