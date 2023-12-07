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
pub unsafe extern "C" fn UnInit(fnc: u32) -> i32 {
    crate::UnInit_impl(fnc)
}
