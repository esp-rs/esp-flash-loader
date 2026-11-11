#![no_std]
#![no_main]
#![cfg_attr(target_arch = "xtensa", feature(asm_experimental_arch))]

#[macro_export]
macro_rules! dprintln {
    () => {
        ufmt::uwriteln!($crate::micro_rtt::RttLog, "").ok()
    };
    ($fmt:literal) => {
        ufmt::uwriteln!($crate::micro_rtt::RttLog, $fmt).ok()
    };
    ($fmt:literal, $($arg:tt)*) => {
        ufmt::uwriteln!($crate::micro_rtt::RttLog, $fmt, $($arg)*).ok()
    };
}

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
#[cfg_attr(feature = "esp32h21", path = "chip/esp32h21.rs")]
#[cfg_attr(feature = "esp32h4", path = "chip/esp32h4.rs")]
#[cfg_attr(feature = "esp32p4", path = "chip/esp32p4.rs")]
#[cfg_attr(feature = "esp32s31", path = "chip/esp32s31.rs")]
mod chip;
mod efuse;
mod rom;

// Define necessary functions for flash loader
//
// These are taken from the [ARM CMSIS-Pack documentation]
//
// [ARM CMSIS-Pack documentation]: https://arm-software.github.io/CMSIS_5/Pack/html/algorithmFunc.html

use core::mem::MaybeUninit;

use chip::CpuSaveState;
use panic_never as _;

use crate::{decompressor::Decompressor, signature::SignatureEntry};

#[cfg_attr(any(target_arch = "xtensa"), path = "api_xtensa.rs")]
mod api;
mod decompressor;
mod flash;
mod md5;
mod micro_rtt;
mod properties;
mod signature;
mod tinfl;

#[cfg(not(any(target_arch = "xtensa", target_arch = "riscv32")))]
compile_error!("specify the target with `--target`");

const ERROR_BASE_INTERNAL: i32 = -1000;
const ERROR_BASE_TINFL: i32 = -2000;
const ERROR_BASE_FLASH: i32 = -4000;

// Reserve memory for the data buffer so that we can use `nm` to
// get its location and we ensure nothing gets placed on top of it.
#[unsafe(no_mangle)]
#[used]
static mut PAGE_BUFFER: [MaybeUninit<u8>; 32 * 1024] = [MaybeUninit::uninit(); 32 * 1024];

struct FlasherState {
    inited: bool,
    saved_cpu_state: CpuSaveState,
    decompressor: Decompressor,
    signature_buffer: [u8; 4096],
    md5: md5::Md5Context,
}

static mut STATE: FlasherState = FlasherState {
    inited: false,
    saved_cpu_state: CpuSaveState::new(),
    decompressor: Decompressor::new(),
    signature_buffer: [0; 4096],
    md5: md5::Md5Context::new(),
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

/// Signs `count` entries of the signature table at `table`.
///
/// Returns the number of entries done, or a negative error code.
#[no_mangle]
pub unsafe extern "C" fn Signatures_md5_impl(table: *mut u8, count: u32) -> i32 {
    let Some(state) = state() else {
        return ERROR_BASE_INTERNAL - 1;
    };

    dprintln!("SIGN {} ranges", count);

    let table = core::slice::from_raw_parts_mut(table as *mut SignatureEntry, count as usize);
    for entry in table {
        if let Err(error) = entry.sign(&mut state.md5, &mut state.signature_buffer) {
            return error;
        }
    }

    count as i32
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

#[no_mangle]
pub unsafe extern "C" fn FlashSize_impl() -> i32 {
    if state().is_none() {
        return ERROR_BASE_INTERNAL - 1;
    };

    flash::flash_size() as i32
}

#[no_mangle]
pub unsafe extern "C" fn ChipRevision_impl() -> i32 {
    if state().is_none() {
        return ERROR_BASE_INTERNAL - 1;
    };

    let revision = efuse::read_chip_revision() as i32;

    dprintln!("CHIP REVISION: {}", revision);

    revision
}
