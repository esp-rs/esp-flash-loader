#![no_std]
#![no_main]
#![cfg_attr(target_arch = "xtensa", feature(asm_experimental_arch))]

// TODO: implement clock frequency setting for newer chips

#[cfg_attr(feature = "esp32", path = "chip/esp32.rs")]
#[cfg_attr(feature = "esp32s2", path = "chip/esp32s2.rs")]
#[cfg_attr(feature = "esp32s3", path = "chip/esp32s3.rs")]
#[cfg_attr(feature = "esp32c2", path = "chip/esp32c2.rs")]
#[cfg_attr(feature = "esp32c3", path = "chip/esp32c3.rs")]
#[cfg_attr(feature = "esp32c5", path = "chip/esp32c5.rs")]
#[cfg_attr(feature = "esp32c6", path = "chip/esp32c6.rs")]
#[cfg_attr(feature = "esp32c61", path = "chip/esp32c61.rs")]
#[cfg_attr(feature = "esp32h2", path = "chip/esp32h2.rs")]
#[cfg_attr(feature = "esp32p4", path = "chip/esp32p4.rs")]
mod chip;
mod efuse;
mod rom;

// Define necessary functions for flash loader
//
// These are taken from the [ARM CMSIS-Pack documentation]
//
// [ARM CMSIS-Pack documentation]: https://arm-software.github.io/CMSIS_5/Pack/html/algorithmFunc.html

use chip::CpuSaveState;
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

struct FlasherState {
    inited: bool,
    saved_cpu_state: CpuSaveState,
    decompressor: Decompressor,
    read_buffer: [u8; 256],
}

static mut STATE: FlasherState = FlasherState {
    inited: false,
    saved_cpu_state: CpuSaveState::new(),
    decompressor: Decompressor::new(),
    read_buffer: [0; 256],
};

fn state() -> Option<&'static mut FlasherState> {
    #[allow(static_mut_refs)]
    let state = unsafe { &mut STATE };

    if state.inited {
        Some(state)
    } else {
        None
    }
}

fn init_state() -> &'static mut FlasherState {
    #[allow(static_mut_refs)]
    let state = unsafe { &mut STATE };

    state.decompressor = Decompressor::new();
    state.inited = true;

    state
}

fn init_bss() {
    extern "C" {
        static mut _bss_start: u32;
        static mut _bss_end: u32;
    }

    let start = (&raw const _bss_start) as u32;
    let end = (&raw const _bss_end) as u32;

    for addr in (start..end).step_by(4) {
        unsafe {
            (addr as *mut u32).write_volatile(0);
        }
    }
}

/// Setup the device for the flashing process.
#[no_mangle]
pub unsafe extern "C" fn Init_impl(_adr: u32, _clk: u32, _fnc: u32) -> i32 {
    init_bss();
    dprintln!("INIT");

    rom::init_rom_data();

    let state = init_state();
    state.saved_cpu_state.set_max_cpu_clock();

    flash::attach()
}

/// Erase the sector at the given address in flash
///
/// Returns 0 on success, 1 on failure.
#[no_mangle]
pub unsafe extern "C" fn EraseSector_impl(adr: u32) -> i32 {
    if state().is_none() {
        return ERROR_BASE_INTERNAL - 1;
    };
    flash::erase_block(adr)
}

#[no_mangle]
pub unsafe extern "C" fn EraseChip_impl() -> i32 {
    if state().is_none() {
        return ERROR_BASE_INTERNAL - 1;
    };
    flash::erase_chip()
}

#[no_mangle]
pub unsafe extern "C" fn ProgramPage_impl(adr: u32, sz: u32, buf: *const u8) -> i32 {
    let Some(state) = state() else {
        return ERROR_BASE_INTERNAL - 1;
    };

    if (buf as u32) % 4 != 0 {
        dprintln!("ERROR buf not word aligned");
        return ERROR_BASE_INTERNAL - 5;
    }

    dprintln!("PROGRAM {} bytes @ {}", sz, adr);

    let input = core::slice::from_raw_parts(buf, sz as usize);

    state.decompressor.program(adr, input)
}

#[no_mangle]
pub unsafe extern "C" fn Verify_impl(adr: u32, sz: u32, buf: *const u8) -> i32 {
    let Some(state) = state() else {
        return ERROR_BASE_INTERNAL - 1;
    };

    if (buf as u32) % 4 != 0 {
        dprintln!("ERROR buf not word aligned");
        return ERROR_BASE_INTERNAL - 5;
    }

    dprintln!("PROGRAM {} bytes @ {}", sz, adr);

    let input = core::slice::from_raw_parts(buf, sz as usize);

    state.decompressor.verify(adr, input)
}

#[no_mangle]
pub unsafe extern "C" fn ReadFlash_impl(adr: u32, sz: u32, buf: *mut u8) -> i32 {
    if state().is_none() {
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
pub unsafe extern "C" fn BlankCheck_impl(adr: u32, sz: u32, pat: u8) -> i32 {
    let Some(state) = state() else {
        return ERROR_BASE_INTERNAL - 1;
    };

    for i in (0..sz).step_by(state.read_buffer.len()) {
        if crate::flash::read_flash(adr + i as u32, &mut state.read_buffer) != 0 {
            return ERROR_BASE_INTERNAL - 2;
        }
        let mut idx = 0;
        while idx < state.read_buffer.len() {
            if state.read_buffer[idx] != pat {
                return ERROR_BASE_INTERNAL - 3;
            }
            idx += 1;
        }
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn UnInit_impl(fnc: u32) -> i32 {
    let Some(state) = state() else {
        return ERROR_BASE_INTERNAL - 1;
    };

    state.saved_cpu_state.restore();
    state.inited = false;

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
    image_start: Option<u32>,
    offset: u32,
    remaining_compressed: usize,
}

impl Decompressor {
    pub const fn new() -> Self {
        Self {
            image_start: None,
            offset: 0,
            output: OutBuffer::new(),
            remaining_compressed: 0,
            decompressor: TinflDecompressor::new(),
        }
    }

    fn reinit(&mut self, address: u32, compressed: u32) {
        self.image_start = Some(address);
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
        let address = self.image_start.unwrap_or(0) + offset;

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
        if self.image_start != Some(address) {
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
