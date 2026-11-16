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
        data_start: 0x4003700C,
        data_end: 0x400371E0,
        bss_start: 0x400371E0,
        bss_end: 0x40037310,
    },
    RomDataTable {
        min_revision: 100,
        data_start: 0x40038A4C,
        data_end: 0x40038C38,
        bss_start: 0x40038C38,
        bss_end: 0x40038D78,
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
    saved_mspi_clk_conf: u32,
    saved_spi1_clock: u32,
    flash_clock_modified: bool,
}

impl CpuSaveState {
    const PCR: u32 = 0x6009_6000;
    const MSPI_CLK_CONF: *mut u32 = (Self::PCR + 0x28) as *mut u32;
    const SYSCLK_CONF: *const u32 = (Self::PCR + 0xE8) as *const u32;
    const CPU_FREQ_CONF: *mut u32 = (Self::PCR + 0xF0) as *mut u32;
    const BUS_CLK_UPDATE: *mut u32 = (Self::PCR + 0x120) as *mut u32;
    const SPI1_CLOCK: *mut u32 = (MEM_SPI.base + 0x14) as *mut u32;

    const SOC_CLK_SEL_SHIFT: u32 = 16;
    const SOC_CLK_SEL_PLL_F160M: u32 = 2;
    const CPU_DIV_NUM_MASK: u32 = 0xFF;
    const BUS_CLOCK_UPDATE: u32 = 1 << 0;
    const CPU_MHZ: u32 = 160;

    // MSPI_FAST_DIV_NUM and MSPI_FUNC_CLK_SEL. Zero selects XTAL_CLK, undivided.
    const MSPI_CLK_SRC_MASK: u32 = 0x3FF;
    const SPI1_CLK_EQU_SYSCLK: u32 = 1 << 31;

    pub const fn new() -> Self {
        CpuSaveState {
            saved_cpu_div: 0,
            saved_ticks_per_us: 0,
            modified: false,
            saved_mspi_clk_conf: 0,
            saved_spi1_clock: 0,
            flash_clock_modified: false,
        }
    }

    /// Runs SPI1 at the 40 MHz XTAL_CLK instead of the reset default XTAL_CLK / 4.
    pub fn configure_flash(&mut self) {
        unsafe {
            self.saved_mspi_clk_conf = Self::MSPI_CLK_CONF.read_volatile();
            self.saved_spi1_clock = Self::SPI1_CLOCK.read_volatile();
            self.flash_clock_modified = true;

            Self::MSPI_CLK_CONF.write_volatile(self.saved_mspi_clk_conf & !Self::MSPI_CLK_SRC_MASK);
            Self::SPI1_CLOCK.write_volatile(Self::SPI1_CLK_EQU_SYSCLK);
        }
    }

    fn restore_flash(&self) {
        if !self.flash_clock_modified {
            return;
        }
        unsafe {
            Self::SPI1_CLOCK.write_volatile(self.saved_spi1_clock);
            Self::MSPI_CLK_CONF.write_volatile(self.saved_mspi_clk_conf);
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
        // Only the CPU divider changes. AHB_CLK is divided from the root clock, so it stays
        // within its limit and CPU_CLK stays an integer multiple of it.
        let soc_clk_sel =
            unsafe { Self::SYSCLK_CONF.read_volatile() } >> Self::SOC_CLK_SEL_SHIFT & 3;
        if soc_clk_sel != Self::SOC_CLK_SEL_PLL_F160M {
            return;
        }

        self.saved_cpu_div =
            unsafe { Self::CPU_FREQ_CONF.read_volatile() } & Self::CPU_DIV_NUM_MASK;
        self.saved_ticks_per_us = unsafe { ets_get_cpu_frequency() };
        self.modified = true;

        Self::set_cpu_div(0);
        unsafe { ets_update_cpu_frequency(Self::CPU_MHZ) };
    }

    pub fn restore(&self) {
        self.restore_flash();

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
