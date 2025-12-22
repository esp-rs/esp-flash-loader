use crate::{
    efuse::{read_field, EfuseInfo},
    rom::{RomDataTable, RomDataTables},
};

// Max of 16MB
pub const MAX_FLASH_SIZE: u32 = 0x1000000;

pub const ROM_DATA_TABLES: RomDataTables = &[RomDataTable {
    min_revision: 0,
    data_start: 0x4000D4F8,
    data_end: 0x4000D5C8,
    bss_start: 0x4000D5D0,
    bss_end: 0x4000D66C,
}];

pub const ROM_TABLE_ENTRY_SIZE: u32 = 16;

pub const EFUSE_INFO: EfuseInfo = EfuseInfo {
    block0: 0x3FF5_A000,
    block_sizes: &[7 + 7, 8, 8, 8],
};

pub struct CpuSaveState {
    saved_cpu_per_conf_reg: u32,
    saved_sysclk_conf_reg: u32,
}

impl CpuSaveState {
    const SYSTEM_CPU_PER_CONF_REG: *mut u32 = 0x3FF0003C as *mut u32;
    const SYSTEM_CPUPERIOD_SEL_M: u32 = 3;
    const SYSTEM_CPUPERIOD_MAX: u32 = 2;

    const SYSTEM_SYSCLK_CONF_REG: *mut u32 = 0x3FF48070 as *mut u32;
    const SYSTEM_SOC_CLK_SEL_M: u32 = 3 << 27;
    const SYSTEM_SOC_CLK_MAX: u32 = 1 << 27;

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
    let eco_bit0 = read_field::<0, 111, 1>() as u32;
    let eco_bit1 = read_field::<0, 180, 1>() as u32;
    let apb_ctrl_date = (0x3ff6_6000 + 0x7c) as *const u32;
    let eco_bit2 = (unsafe { *apb_ctrl_date } & 0x80000000) >> 31;

    match (eco_bit2 << 2) | (eco_bit1 << 1) | eco_bit0 {
        1 => 1,
        3 => 2,
        7 => 3,
        _ => 0,
    }
}

pub fn minor_chip_version() -> u8 {
    read_field::<0, 184, 2>()
}
