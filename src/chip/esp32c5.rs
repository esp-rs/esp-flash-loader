use crate::{
    efuse::{read_field, EfuseInfo},
    flash::MemSpi,
    rom::{RomDataTable, RomDataTables},
};

// Max of 32MB
pub const MAX_FLASH_SIZE: u32 = 0x2000000;

pub const ROM_DATA_TABLES: RomDataTables = &[
    RomDataTable {
        min_revision: 0,
        data_start: 0x400478A8,
        data_end: 0x40047A7C,
        bss_start: 0x40047A7C,
        bss_end: 0x40047BAC,
    },
    RomDataTable {
        min_revision: 100,
        data_start: 0x4003B154,
        data_end: 0x4003B340,
        bss_start: 0x4003B340,
        bss_end: 0x4003B480,
    },
];

pub const ROM_TABLE_ENTRY_SIZE: u32 = 12;

pub const EFUSE_INFO: EfuseInfo = EfuseInfo {
    block0: 0x600B_4800 + 0x2C,
    block_sizes: &[6, 6, 8, 8, 8, 8, 8, 8, 8, 8, 8],
};

pub const MEM_SPI: MemSpi = MemSpi {
    base: 0x6000_3000,
    cmd: 0x00,
    addr: 0x04,
    ctrl: 0x08,
    user: 0x18,
    user1: 0x1C,
    user2: 0x20,
    miso_dlen: 0x28,
    data_buf_0: 0x58,
};

unsafe extern "C" {
    fn ets_get_cpu_frequency() -> u32;
    fn ets_update_cpu_frequency(ticks_per_us: u32);
}

pub struct CpuSaveState {
    saved_cpu_div: u32,
    saved_ticks_per_us: u32,
    modified: bool,
}

impl CpuSaveState {
    const PCR: u32 = 0x6009_6000;
    const SYSCLK_CONF: *const u32 = (Self::PCR + 0x110) as *const u32;
    const CPU_FREQ_CONF: *mut u32 = (Self::PCR + 0x118) as *mut u32;
    const BUS_CLK_UPDATE: *mut u32 = (Self::PCR + 0x144) as *mut u32;

    const SOC_CLK_SEL_SHIFT: u32 = 16;
    const SOC_CLK_SEL_PLL_F160M: u32 = 2;
    const SOC_CLK_SEL_PLL_F240M: u32 = 3;
    const CPU_DIV_NUM_MASK: u32 = 0xFF;
    const BUS_CLOCK_UPDATE: u32 = 1 << 0;

    pub const fn new() -> Self {
        CpuSaveState {
            saved_cpu_div: 0,
            saved_ticks_per_us: 0,
            modified: false,
        }
    }

    fn set_cpu_div(div: u32) {
        unsafe {
            Self::CPU_FREQ_CONF.write_volatile(
                (Self::CPU_FREQ_CONF.read_volatile() & !Self::CPU_DIV_NUM_MASK) | div,
            );
            Self::BUS_CLK_UPDATE.write_volatile(Self::BUS_CLOCK_UPDATE);
            while Self::BUS_CLK_UPDATE.read_volatile() & Self::BUS_CLOCK_UPDATE != 0 {}
        }
    }

    pub fn set_max_cpu_clock(&mut self) {
        // Only the CPU divider changes. Switching between the PLL taps needs the root clock
        // auto-gating workaround, and AHB_CLK is divided from the root clock, so it stays within
        // its limit and CPU_CLK stays an integer multiple of it.
        let soc_clk_sel =
            unsafe { Self::SYSCLK_CONF.read_volatile() } >> Self::SOC_CLK_SEL_SHIFT & 3;
        let cpu_mhz = match soc_clk_sel {
            Self::SOC_CLK_SEL_PLL_F160M => 160,
            Self::SOC_CLK_SEL_PLL_F240M => 240,
            _ => return,
        };

        self.saved_cpu_div =
            unsafe { Self::CPU_FREQ_CONF.read_volatile() } & Self::CPU_DIV_NUM_MASK;
        self.saved_ticks_per_us = unsafe { ets_get_cpu_frequency() };
        self.modified = true;

        Self::set_cpu_div(0);
        unsafe { ets_update_cpu_frequency(cpu_mhz) };
    }

    pub fn configure_flash(&mut self) {}

    pub fn restore(&self) {
        if !self.modified {
            return;
        }
        Self::set_cpu_div(self.saved_cpu_div);
        unsafe { ets_update_cpu_frequency(self.saved_ticks_per_us) };
    }
}

pub fn major_chip_version() -> u8 {
    read_field::<1, 68, 2>()
}

pub fn minor_chip_version() -> u8 {
    read_field::<1, 64, 4>()
}

/// Ensures that data (e.g. constants) are accessed through the data bus.
pub unsafe fn read_via_data_bus(s: &u8) -> u8 {
    unsafe { core::ptr::read(s as *const u8) }
}
