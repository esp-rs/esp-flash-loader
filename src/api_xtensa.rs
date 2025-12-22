// Probe-rs doesn't know how to call a function on the Xtensa architecture. Due to the windowed
// ABI, just jumping to the function address won't work. Instead, we need to use a call<N>
// instruction, which will set up the window increment and then jump to the function address.

// _stack_start is defined in the chip-specific linker files

unsafe extern "C" {
    static _stack_start: u32;
}

core::arch::global_asm!(
    "
    .section .text,\"ax\",@progbits
    .literal STACK_PTR, {_stack_start}
    ",
    _stack_start = sym _stack_start
);

#[unsafe(naked)]
#[unsafe(no_mangle)]
extern "C" fn Init() {
    core::arch::naked_asm!(
        "l32r a1, STACK_PTR",
        "mov.n a6, a2",
        "mov.n a7, a3",
        "mov.n a8, a4",
        "call4 Init_impl",
        "mov.n a2, a6",
        "break 1, 15",
    );
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
extern "C" fn EraseSector() {
    core::arch::naked_asm!(
        "l32r a1, STACK_PTR",
        "mov.n a6, a2",
        "call4 EraseSector_impl",
        "mov.n a2, a6",
        "break 1, 15",
    );
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
extern "C" fn EraseChip() {
    core::arch::naked_asm!(
        "l32r a1, STACK_PTR",
        "call4 EraseChip_impl",
        "mov.n a2, a6",
        "break 1, 15"
    );
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
extern "C" fn ProgramPage() {
    core::arch::naked_asm!(
        "l32r a1, STACK_PTR",
        "mov.n a6, a2",
        "mov.n a7, a3",
        "mov.n a8, a4",
        "call4 ProgramPage_impl",
        "mov.n a2, a6",
        "break 1, 15",
    );
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
extern "C" fn Verify() {
    core::arch::naked_asm!(
        "l32r a1, STACK_PTR",
        "mov.n a6, a2",
        "mov.n a7, a3",
        "mov.n a8, a4",
        "call4 Verify_impl",
        "mov.n a2, a6",
        "break 1, 15",
    );
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
extern "C" fn ReadFlash() {
    core::arch::naked_asm!(
        "l32r a1, STACK_PTR",
        "mov.n a6, a2",
        "mov.n a7, a3",
        "mov.n a8, a4",
        "call4 ReadFlash_impl",
        "mov.n a2, a6",
        "break 1, 15",
    );
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
extern "C" fn UnInit() {
    core::arch::naked_asm!(
        "l32r a1, STACK_PTR",
        "mov.n a6, a2",
        "call4 UnInit_impl",
        "mov.n a2, a6",
        "break 1, 15",
    );
}
