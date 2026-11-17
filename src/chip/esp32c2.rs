use crate::{
    efuse::{read_field, EfuseInfo},
    flash::{rom_write_chunk::RomWriteChunk, MemSpi},
    rom::{RomDataTable, RomDataTables},
};

// Max of 16MB
pub const MAX_FLASH_SIZE: u32 = 0x1000000;

pub const ROM_DATA_TABLES: RomDataTables = &[
    RomDataTable {
        min_revision: 0,
        data_start: 0x40082174,
        data_end: 0x400823F4,
        bss_start: 0x40082404,
        bss_end: 0x400825E4,
    },
    RomDataTable {
        min_revision: 400,
        data_start: 0x40081EEC,
        data_end: 0x4008219C,
        bss_start: 0x400821AC,
        bss_end: 0x400823B0,
    },
];

pub const ROM_TABLE_ENTRY_SIZE: u32 = 16;

pub const EFUSE_INFO: EfuseInfo = EfuseInfo {
    block0: 0x6000_8800 + 0x2C,
    block_sizes: &[8, 12, 32, 32],
};

pub const MEM_SPI: MemSpi = MemSpi {
    base: 0x6000_2000,
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
    write_chunk: RomWriteChunk,
}

impl CpuSaveState {
    const SYSTEM_CPU_PER_CONF_REG: *mut u32 = 0x600C0008 as *mut u32;
    const SYSTEM_CPUPERIOD_SEL_M: u32 = 3;
    const SYSTEM_CPUPERIOD_MAX: u32 = 1;

    const SYSTEM_SYSCLK_CONF_REG: *mut u32 = 0x600C0058 as *mut u32;
    const SYSTEM_SOC_CLK_SEL_M: u32 = 3 << 10;
    const SYSTEM_SOC_CLK_MAX: u32 = 1 << 10;

    const SPI1_CLOCK: *mut u32 = (MEM_SPI.base + 0x14) as *mut u32;
    // 60 MHz / 2
    const SPI1_CLOCK_DIV2: u32 = (1 << 16) | 1;

    pub const fn new() -> Self {
        CpuSaveState {
            saved_cpu_per_conf_reg: 0,
            saved_sysclk_conf_reg: 0,
            saved_spi1_clock: 0,
            flash_clock_modified: false,
            write_chunk: RomWriteChunk::new(),
        }
    }

    /// Runs SPI1 at 30 MHz instead of the reset default of 60 MHz / 4, and makes the ROM write
    /// 64 bytes per command.
    pub fn configure_flash(&mut self) {
        let soc_clk_sel =
            unsafe { Self::SYSTEM_SYSCLK_CONF_REG.read_volatile() } & Self::SYSTEM_SOC_CLK_SEL_M;
        // The MSPI clock is 60 MHz only with the CPU on the PLL.
        if soc_clk_sel == Self::SYSTEM_SOC_CLK_MAX {
            unsafe {
                self.saved_spi1_clock = Self::SPI1_CLOCK.read_volatile();
                self.flash_clock_modified = true;
                Self::SPI1_CLOCK.write_volatile(Self::SPI1_CLOCK_DIV2);
            }
        }
        self.write_chunk.enlarge();
    }

    fn restore_flash(&self) {
        self.write_chunk.restore();
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

pub fn is_data_ram(address: u32) -> bool {
    (0x3FCA_0000..0x3FCE_0000).contains(&address)
}

pub fn major_chip_version() -> u8 {
    read_field::<2, 52, 2>()
}

pub fn minor_chip_version() -> u8 {
    read_field::<2, 48, 4>()
}

/// Ensures that data (e.g. constants) are accessed through the data bus.
pub unsafe fn read_via_data_bus(s: &u8) -> u8 {
    unsafe { core::ptr::read(s as *const u8) }
}
