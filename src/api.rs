/// Setup the device for the flashing process.
#[no_mangle]
pub unsafe extern "C" fn Init(adr: u32, clk: u32, fnc: u32) -> i32 {
    crate::Init_impl(adr, clk, fnc)
}

/// Erase the sector at the given address in flash
///
/// Returns 0 on success, 1 on failure.
#[no_mangle]
pub unsafe extern "C" fn EraseSector(adr: u32) -> i32 {
    crate::EraseSector_impl(adr)
}

#[no_mangle]
pub unsafe extern "C" fn EraseChip() -> i32 {
    crate::EraseChip_impl()
}

#[no_mangle]
pub unsafe extern "C" fn ProgramPage(adr: u32, sz: u32, buf: *const u8) -> i32 {
    crate::ProgramPage_impl(adr, sz, buf)
}

#[no_mangle]
pub unsafe extern "C" fn Verify(adr: u32, sz: u32, buf: *const u8) -> i32 {
    crate::Verify_impl(adr, sz, buf)
}

#[no_mangle]
pub unsafe extern "C" fn BlankCheck(adr: u32, sz: u32, pat: u8) -> i32 {
    crate::BlankCheck_impl(adr, sz, pat)
}

#[no_mangle]
pub unsafe extern "C" fn UnInit(fnc: u32) -> i32 {
    crate::UnInit_impl(fnc)
}

// probe-rs custom functions

#[no_mangle]
pub unsafe extern "C" fn ReadFlash(adr: u32, sz: u32, buf: *mut u8) -> i32 {
    crate::ReadFlash_impl(adr, sz, buf)
}
