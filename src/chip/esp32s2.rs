use crate::{
    efuse::{read_field, EfuseInfo},
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

pub const ROM_TABLE_ENTRY_SIZE: u32 = 12;

pub const EFUSE_INFO: EfuseInfo = EfuseInfo {
    block0: 0x3F41_A000 + 0x2C,
    block_sizes: &[6, 6, 8, 8, 8, 8, 8, 8, 8, 8, 8],
};

pub struct CpuSaveState {
    saved_cpu_per_conf_reg: u32,
    saved_sysclk_conf_reg: u32,
}

impl CpuSaveState {
    const SYSTEM_CPU_PER_CONF_REG: *mut u32 = 0x3F4C0018 as *mut u32;
    const SYSTEM_CPUPERIOD_SEL_M: u32 = 3;
    const SYSTEM_CPUPERIOD_MAX: u32 = 2;

    const SYSTEM_SYSCLK_CONF_REG: *mut u32 = 0x3F4C008C as *mut u32;
    const SYSTEM_SOC_CLK_SEL_M: u32 = 3 << 10;
    const SYSTEM_SOC_CLK_MAX: u32 = 1 << 10;

    pub const fn new() -> Self {
        CpuSaveState {
            saved_cpu_per_conf_reg: 0,
            saved_sysclk_conf_reg: 0,
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
