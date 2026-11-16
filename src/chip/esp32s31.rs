use crate::{
    efuse::{read_field, EfuseInfo},
    flash::MemSpi,
    rom::{RomDataTable, RomDataTables},
};

// Max of 256MB
pub const MAX_FLASH_SIZE: u32 = 0x10000000;

// No ROM data init tables needed; ROM boot already initializes its own data.
pub const ROM_DATA_TABLES: RomDataTables = &[];

// Suppress dead-code warning: ROM_DATA_TABLES is an empty slice but the type
// still requires the element type to be in scope.
const _: RomDataTable = RomDataTable {
    min_revision: 0,
    data_start: 0,
    data_end: 0,
    bss_start: 0,
    bss_end: 0,
};

pub const ROM_TABLE_ENTRY_SIZE: u32 = 12;

pub const EFUSE_INFO: EfuseInfo = EfuseInfo {
    // EFUSE peripheral base = 0x20715000; readable data starts at offset 0x2C
    block0: 0x2071_5000 + 0x2C,
    block_sizes: &[9, 6, 8, 8, 8, 8, 8, 8, 8, 8],
};

pub const MEM_SPI: MemSpi = MemSpi {
    // SPIMEM1 (SPI1 flash controller) base address
    base: 0x2050_1000,
    cmd: 0x00,
    addr: 0x04,
    ctrl: 0x08,
    user: 0x18,
    user1: 0x1C,
    user2: 0x20,
    miso_dlen: 0x28,
    data_buf_0: 0x58,
};

extern "C" {
    fn ets_get_cpu_frequency() -> u32;
    fn ets_update_cpu_frequency(ticks_per_us: u32);
}

pub struct CpuSaveState {
    saved: [u32; 5],
    saved_ticks_per_us: u32,
    modified: bool,
}

impl CpuSaveState {
    const HP_SYS_CLKRST: u32 = 0x2058_7000;
    const SOC_CLK_SEL: *mut u32 = Self::HP_SYS_CLKRST as *mut u32;
    const CPU_FREQ_CTRL0: *mut u32 = (Self::HP_SYS_CLKRST + 0x04) as *mut u32;
    const MEM_FREQ_CTRL0: *mut u32 = (Self::HP_SYS_CLKRST + 0x08) as *mut u32;
    const SYS_FREQ_CTRL0: *mut u32 = (Self::HP_SYS_CLKRST + 0x0C) as *mut u32;
    const APB_FREQ_CTRL0: *mut u32 = (Self::HP_SYS_CLKRST + 0x10) as *mut u32;
    const ROOT_CLK_CTRL0: *mut u32 = (Self::HP_SYS_CLKRST + 0x14) as *mut u32;
    const ANA_PLL_CTRL0: *const u32 = (Self::HP_SYS_CLKRST + 0x174) as *const u32;
    const PMU_HP_ACTIVE_HP_CK_POWER: *const u32 = 0x2070_401C as *const u32;

    const SOC_CLK_DIV_UPDATE: u32 = 1 << 0;
    const CPU_PLL_CAL_END: u32 = 1 << 2;
    const HP_ACTIVE_XPD_PLL: u32 = 1 << 27;

    const SOC_CLK_SEL_CPLL: u32 = 1;
    // The CPLL runs at 320 MHz. MEM_CLK must stay at or below 160 MHz, SYS_CLK at or below
    // 320/3 MHz and APB_CLK at or below 320/6 MHz. Each field holds the divider minus one.
    const CPU_MHZ: u32 = 320;
    const CPU_DIV: u32 = 0;
    const MEM_DIV: u32 = 1;
    const SYS_DIV: u32 = 2;
    const APB_DIV: u32 = 1;

    pub const fn new() -> Self {
        CpuSaveState {
            saved: [0; 5],
            saved_ticks_per_us: 0,
            modified: false,
        }
    }

    fn regs() -> [*mut u32; 5] {
        [
            Self::CPU_FREQ_CTRL0,
            Self::MEM_FREQ_CTRL0,
            Self::SYS_FREQ_CTRL0,
            Self::APB_FREQ_CTRL0,
            Self::SOC_CLK_SEL,
        ]
    }

    fn update_bus_clocks() {
        unsafe {
            Self::ROOT_CLK_CTRL0
                .write_volatile(Self::ROOT_CLK_CTRL0.read_volatile() | Self::SOC_CLK_DIV_UPDATE);
            while Self::ROOT_CLK_CTRL0.read_volatile() & Self::SOC_CLK_DIV_UPDATE != 0 {}
        }
    }

    pub fn set_max_cpu_clock(&mut self) {
        // Leave the clock alone unless the ROM left the CPLL powered and calibrated.
        let cpll_ready = unsafe {
            Self::ANA_PLL_CTRL0.read_volatile() & Self::CPU_PLL_CAL_END != 0
                && Self::PMU_HP_ACTIVE_HP_CK_POWER.read_volatile() & Self::HP_ACTIVE_XPD_PLL != 0
        };
        if !cpll_ready {
            return;
        }

        for (saved, reg) in self.saved.iter_mut().zip(Self::regs()) {
            *saved = unsafe { reg.read_volatile() };
        }
        self.saved_ticks_per_us = unsafe { ets_get_cpu_frequency() };
        self.modified = true;

        let div_fields = |reg: *mut u32, div: u32| unsafe {
            // Integer divider, fractional part 0/0.
            reg.write_volatile((reg.read_volatile() & !0x3FFF) | div);
        };
        div_fields(Self::CPU_FREQ_CTRL0, Self::CPU_DIV);
        unsafe {
            Self::MEM_FREQ_CTRL0
                .write_volatile((Self::MEM_FREQ_CTRL0.read_volatile() & !1) | Self::MEM_DIV);
        }
        div_fields(Self::SYS_FREQ_CTRL0, Self::SYS_DIV);
        div_fields(Self::APB_FREQ_CTRL0, Self::APB_DIV);
        unsafe {
            Self::SOC_CLK_SEL
                .write_volatile((Self::SOC_CLK_SEL.read_volatile() & !3) | Self::SOC_CLK_SEL_CPLL);
        }
        Self::update_bus_clocks();
        unsafe { ets_update_cpu_frequency(Self::CPU_MHZ) };
    }

    pub fn configure_flash(&mut self) {}

    pub fn restore(&self) {
        if !self.modified {
            return;
        }
        for (saved, reg) in self.saved.iter().zip(Self::regs()) {
            unsafe { reg.write_volatile(*saved) };
        }
        Self::update_bus_clocks();
        unsafe { ets_update_cpu_frequency(self.saved_ticks_per_us) };
    }
}

// EFUSE_BLK1, bit 118, 2 bits (WAFER_VERSION_MAJOR)
pub fn major_chip_version() -> u8 {
    read_field::<1, 118, 2>()
}

// EFUSE_BLK1, bit 114, 4 bits (WAFER_VERSION_MINOR)
pub fn minor_chip_version() -> u8 {
    read_field::<1, 114, 4>()
}

/// Ensures that data (e.g. constants) are accessed through the data bus.
pub unsafe fn read_via_data_bus(s: &u8) -> u8 {
    unsafe { core::ptr::read(s as *const u8) }
}
