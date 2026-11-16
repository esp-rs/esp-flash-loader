use crate::{
    efuse::{read_field, EfuseInfo},
    flash::MemSpi,
    rom::{RomDataTable, RomDataTables},
};

// Max of 1GB
pub const MAX_FLASH_SIZE: u32 = 0x40000000;

pub const ROM_DATA_TABLES: RomDataTables = &[RomDataTable {
    min_revision: 0,
    data_start: 0x4001BD64,
    data_end: 0x4001BE34,
    bss_start: 0x4001BE34,
    bss_end: 0x4001BED0,
}];

pub const ROM_TABLE_ENTRY_SIZE: u32 = 16;

pub const EFUSE_INFO: EfuseInfo = EfuseInfo {
    block0: 0x3F41_A000 + 0x2C,
    block_sizes: &[6, 6, 8, 8, 8, 8, 8, 8, 8, 8, 8],
};

pub const MEM_SPI: MemSpi = MemSpi {
    base: 0x3f40_2000,
    cmd: 0x00,
    addr: 0x04,
    ctrl: 0x08,
    user: 0x18,
    user1: 0x1C,
    user2: 0x20,
    miso_dlen: 0x28,
    data_buf_0: 0x58,
};

pub struct CpuSaveState {
    saved_cpu_per_conf_reg: u32,
    saved_sysclk_conf_reg: u32,
    saved_spi1_clock: u32,
    flash_clock_modified: bool,
}

impl CpuSaveState {
    const SYSTEM_CPU_PER_CONF_REG: *mut u32 = 0x3F4C0018 as *mut u32;
    const SYSTEM_CPUPERIOD_SEL_M: u32 = 3;
    const SYSTEM_CPUPERIOD_MAX: u32 = 2;

    const SYSTEM_SYSCLK_CONF_REG: *mut u32 = 0x3F4C008C as *mut u32;
    const SYSTEM_SOC_CLK_SEL_M: u32 = 3 << 10;
    const SYSTEM_SOC_CLK_MAX: u32 = 1 << 10;

    const SPI1_CLOCK: *mut u32 = (MEM_SPI.base + 0x14) as *mut u32;
    const SPI_MEM_CORE_CLK_SEL_REG: *const u32 = 0x3F40_30E0 as *const u32;
    const SPI01_CLK_SEL_M: u32 = 3;
    // 80 MHz / 2
    const SPI1_CLOCK_DIV2: u32 = (1 << 16) | 1;

    pub const fn new() -> Self {
        CpuSaveState {
            saved_cpu_per_conf_reg: 0,
            saved_sysclk_conf_reg: 0,
            saved_spi1_clock: 0,
            flash_clock_modified: false,
        }
    }

    /// Runs SPI1 at 40 MHz instead of the reset default of 80 MHz / 4.
    pub fn configure_flash(&mut self) {
        let soc_clk_sel =
            unsafe { Self::SYSTEM_SYSCLK_CONF_REG.read_volatile() } & Self::SYSTEM_SOC_CLK_SEL_M;
        let spi01_clk_sel =
            unsafe { Self::SPI_MEM_CORE_CLK_SEL_REG.read_volatile() } & Self::SPI01_CLK_SEL_M;
        // The SPI0/1 core clock is 80 MHz only with the CPU on the PLL.
        if soc_clk_sel != Self::SYSTEM_SOC_CLK_MAX || spi01_clk_sel != 0 {
            return;
        }
        unsafe {
            self.saved_spi1_clock = Self::SPI1_CLOCK.read_volatile();
            self.flash_clock_modified = true;
            Self::SPI1_CLOCK.write_volatile(Self::SPI1_CLOCK_DIV2);
        }
    }

    fn restore_flash(&self) {
        if self.flash_clock_modified {
            unsafe { Self::SPI1_CLOCK.write_volatile(self.saved_spi1_clock) };
        }
    }

    pub fn set_max_cpu_clock(&mut self) {
        self.saved_cpu_per_conf_reg = unsafe { Self::SYSTEM_CPU_PER_CONF_REG.read_volatile() };
        self.saved_sysclk_conf_reg = unsafe { Self::SYSTEM_SYSCLK_CONF_REG.read_volatile() };

        unsafe {
            Self::SYSTEM_CPU_PER_CONF_REG.write_volatile(
                (self.saved_cpu_per_conf_reg & !Self::SYSTEM_CPUPERIOD_SEL_M)
                    | Self::SYSTEM_CPUPERIOD_MAX,
            );
            Self::SYSTEM_SYSCLK_CONF_REG.write_volatile(
                (self.saved_sysclk_conf_reg & !Self::SYSTEM_SOC_CLK_SEL_M)
                    | Self::SYSTEM_SOC_CLK_MAX,
            );
        }
    }

    pub fn restore(&self) {
        self.restore_flash();
        unsafe { Self::SYSTEM_SYSCLK_CONF_REG.write_volatile(self.saved_sysclk_conf_reg) };
        unsafe { Self::SYSTEM_CPU_PER_CONF_REG.write_volatile(self.saved_cpu_per_conf_reg) };
    }
}

pub fn major_chip_version() -> u8 {
    read_field::<1, 114, 2>()
}

pub fn minor_chip_version() -> u8 {
    let lo = read_field::<1, 132, 3>();
    let hi = read_field::<1, 116, 1>();

    hi << 3 | lo
}

/// Ensures that data (e.g. constants) are accessed through the data bus.
pub unsafe fn read_via_data_bus(s: &u8) -> u8 {
    // SRAM1
    const DBUS_START: usize = 0x3FFB_8000;
    const DBUS_END: usize = 0x4000_0000;
    const IBUS_START: usize = 0x4002_8000;

    let addr = s as *const u8 as usize;
    if addr >= DBUS_START && addr < DBUS_END {
        *s
    } else {
        let ptr = addr - IBUS_START + DBUS_START;
        unsafe { core::ptr::read(ptr as *const u8) }
    }
}
