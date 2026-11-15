//! Programs the flash on the APP core, so that the host can load the next page while the APP core
//! writes the current one.
//!
//! The host halts the PRO core to access memory. The PRO core only waits for the APP core, so the
//! halt does not stop the programming.

use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};

use crate::program_page_on_this_core;

#[cfg(feature = "esp32s3")]
mod control {
    const SYSTEM_CORE_1_CONTROL_0_REG: *mut u32 = 0x600C_0000 as *mut u32;
    const CORE_1_RUNSTALL: u32 = 1 << 0;
    const CORE_1_CLKGATE_EN: u32 = 1 << 1;
    const CORE_1_RESETING: u32 = 1 << 2;

    pub unsafe fn release() {
        let reg = SYSTEM_CORE_1_CONTROL_0_REG;
        reg.write_volatile(reg.read_volatile() | CORE_1_CLKGATE_EN);
        reg.write_volatile(reg.read_volatile() & !CORE_1_RUNSTALL);
        reg.write_volatile(reg.read_volatile() | CORE_1_RESETING);
        reg.write_volatile(reg.read_volatile() & !CORE_1_RESETING);
    }

    pub unsafe fn hold() {
        let reg = SYSTEM_CORE_1_CONTROL_0_REG;
        reg.write_volatile(reg.read_volatile() | CORE_1_RUNSTALL | CORE_1_RESETING);
    }
}

#[cfg(feature = "esp32")]
mod control {
    const DPORT_APPCPU_CTRL_A_REG: *mut u32 = 0x3FF0_002C as *mut u32;
    const DPORT_APPCPU_CTRL_B_REG: *mut u32 = 0x3FF0_0030 as *mut u32;
    const DPORT_APPCPU_CTRL_C_REG: *mut u32 = 0x3FF0_0034 as *mut u32;
    const APPCPU_RESETTING: u32 = 1 << 0;
    const APPCPU_CLKGATE_EN: u32 = 1 << 0;
    const APPCPU_RUNSTALL: u32 = 1 << 0;

    pub unsafe fn release() {
        let (a, b, c) = (
            DPORT_APPCPU_CTRL_A_REG,
            DPORT_APPCPU_CTRL_B_REG,
            DPORT_APPCPU_CTRL_C_REG,
        );
        b.write_volatile(b.read_volatile() | APPCPU_CLKGATE_EN);
        c.write_volatile(c.read_volatile() & !APPCPU_RUNSTALL);
        a.write_volatile(a.read_volatile() | APPCPU_RESETTING);
        a.write_volatile(a.read_volatile() & !APPCPU_RESETTING);
    }

    pub unsafe fn hold() {
        let (a, c) = (DPORT_APPCPU_CTRL_A_REG, DPORT_APPCPU_CTRL_C_REG);
        c.write_volatile(c.read_volatile() | APPCPU_RUNSTALL);
        a.write_volatile(a.read_volatile() | APPCPU_RESETTING);
    }
}

const STACK_SIZE: usize = 8 * 1024;

#[repr(C, align(16))]
struct Stack([u8; STACK_SIZE]);

static mut STACK: Stack = Stack([0; STACK_SIZE]);

/// `ets_delay_us` counts with the ROM's tick rate, which can be lower than the CPU clock, so the
/// real timeout can be several times shorter.
const START_TIMEOUT_US: u32 = 100_000;
const START_POLL_US: u32 = 10;

static STARTED: AtomicBool = AtomicBool::new(false);
/// Set when the APP core didn't start. The PRO core then programs until `finish`.
static SINGLE_CORE: AtomicBool = AtomicBool::new(false);
/// Set by the APP core when it reaches `app_core_main`.
static RUNNING: AtomicBool = AtomicBool::new(false);
/// The number of the last page the PRO core handed over.
static POSTED: AtomicU32 = AtomicU32::new(0);
/// The number of the last page the APP core finished.
static DONE: AtomicU32 = AtomicU32::new(0);
/// The result of the last page the APP core finished.
static RESULT: AtomicI32 = AtomicI32::new(0);

static JOB_ADDRESS: AtomicU32 = AtomicU32::new(0);
static JOB_SIZE: AtomicU32 = AtomicU32::new(0);
static JOB_BUFFER: AtomicU32 = AtomicU32::new(0);

extern "C" {
    fn ets_set_appcpu_boot_addr(address: u32);
    fn ets_delay_us(us: u32);
}

/// Has the APP core program `data`, and returns the result. Programs on this core if the APP
/// core doesn't start.
pub fn program(address: u32, data: &[u8]) -> i32 {
    if SINGLE_CORE.load(Ordering::Relaxed) {
        return program_page_on_this_core(address, data);
    }
    if !STARTED.load(Ordering::Acquire) && !start() {
        crate::dprintln!("APP core did not start, programming on this core");
        SINGLE_CORE.store(true, Ordering::Relaxed);
        return program_page_on_this_core(address, data);
    }

    JOB_ADDRESS.store(address, Ordering::Relaxed);
    JOB_SIZE.store(data.len() as u32, Ordering::Relaxed);
    JOB_BUFFER.store(data.as_ptr() as u32, Ordering::Relaxed);
    POSTED.store(
        POSTED.load(Ordering::Relaxed).wrapping_add(1),
        Ordering::Release,
    );

    wait_idle();

    RESULT.load(Ordering::Acquire)
}

/// Stops the APP core.
pub fn finish() {
    if STARTED.load(Ordering::Acquire) {
        stop();
    }
    SINGLE_CORE.store(false, Ordering::Relaxed);
}

fn wait_idle() {
    while DONE.load(Ordering::Acquire) != POSTED.load(Ordering::Relaxed) {}
}

/// Starts the APP core, and returns whether it reached `app_core_main` in time.
fn start() -> bool {
    POSTED.store(0, Ordering::Relaxed);
    DONE.store(0, Ordering::Relaxed);
    RESULT.store(0, Ordering::Relaxed);
    RUNNING.store(false, Ordering::Relaxed);

    unsafe {
        ets_set_appcpu_boot_addr(app_core_entry as *const () as u32);
        control::release();
    }

    STARTED.store(true, Ordering::Release);

    for _ in 0..START_TIMEOUT_US / START_POLL_US {
        if RUNNING.load(Ordering::Acquire) {
            return true;
        }
        unsafe { ets_delay_us(START_POLL_US) };
    }

    stop();
    false
}

fn stop() {
    unsafe {
        control::hold();
        ets_set_appcpu_boot_addr(0);
    }

    STARTED.store(false, Ordering::Release);
}

/// The ROM calls this on the APP core. It runs on the stack of the ROM until it switches to
/// `STACK`, so it must not use locals after the switch.
unsafe extern "C" fn app_core_entry() -> ! {
    let top = (&raw mut STACK as *mut u8).add(STACK_SIZE - 64);

    core::arch::asm!("wsr.intenable {0}", "rsync", in(reg) 0);
    core::arch::asm!("movi a0, 0", "mov sp, {0}", in(reg) top, options(nostack));

    app_core_main()
}

#[inline(never)]
fn app_core_main() -> ! {
    RUNNING.store(true, Ordering::Release);

    let mut done = 0u32;
    loop {
        let posted = POSTED.load(Ordering::Acquire);
        if posted == done {
            continue;
        }

        let address = JOB_ADDRESS.load(Ordering::Relaxed);
        let size = JOB_SIZE.load(Ordering::Relaxed) as usize;
        let buffer = JOB_BUFFER.load(Ordering::Relaxed) as *const u8;
        let data = unsafe { core::slice::from_raw_parts(buffer, size) };

        RESULT.store(program_page_on_this_core(address, data), Ordering::Relaxed);

        done = posted;
        DONE.store(done, Ordering::Release);
    }
}
