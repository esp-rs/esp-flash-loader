use core::arch::asm;

// Probe-rs doesn't know how to call a function on the Xtensa architecture. Due to the windowed
// ABI, just jumping to the function address won't work. Instead, we need to use a call<N>
// instruction, which will set up the window increment and then jump to the function address.

#[cfg(feature = "esp32s3")]
#[no_mangle]
// End of SRAM1 - DATA_CACHE_SIZE
static STACK_PTR: u32 = 0x3FCD_0000;

/// Setup the device for the flashing process.
#[no_mangle]
#[naked]
pub unsafe extern "C" fn Init(adr: u32, clk: u32, fnc: u32) -> i32 {
    asm!(
        "
        .global Init_impl
        l32r a1, STACK_PTR
        mov.n a6, a2
        mov.n a7, a3
        mov.n a8, a4
        call4 Init_impl
        mov.n a2, a6
        ",
        options(noreturn)
    )
}

/// Erase the sector at the given address in flash
///
/// Returns 0 on success, 1 on failure.
#[no_mangle]
#[naked]
pub unsafe extern "C" fn EraseSector(adr: u32) -> i32 {
    asm!(
        "
        .global EraseSector_impl
        l32r a1, STACK_PTR
        mov.n a6, a2
        call4 EraseSector_impl
        mov.n a2, a6
        ",
        options(noreturn)
    )
}

#[no_mangle]
#[naked]
pub unsafe extern "C" fn EraseChip() -> i32 {
    asm!(
        "
        .global EraseChip_impl
        l32r a1, STACK_PTR
        call4 EraseChip_impl
        mov.n a2, a6
        ",
        options(noreturn)
    )
}

#[no_mangle]
#[naked]
pub unsafe extern "C" fn ProgramPage(adr: u32, sz: u32, buf: *const u8) -> i32 {
    asm!(
        "
        .global ProgramPage_impl
        l32r a1, STACK_PTR
        mov.n a6, a2
        mov.n a7, a3
        mov.n a8, a4
        call4 ProgramPage_impl
        mov.n a2, a6
        ",
        options(noreturn)
    )
}

#[no_mangle]
#[naked]
pub unsafe extern "C" fn UnInit(fnc: u32) -> i32 {
    asm!(
        "
        .global UnInit_impl
        l32r a1, STACK_PTR
        mov.n a6, a2
        call4 UnInit_impl
        mov.n a2, a6
        ",
        options(noreturn)
    )
}
