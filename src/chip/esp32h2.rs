use crate::{
    efuse::{read_field, EfuseInfo},
    flash::MemSpi,
    rom::{RomDataTable, RomDataTables},
};

// Max of 16MB
pub const MAX_FLASH_SIZE: u32 = 0x1000000;

pub const ROM_DATA_TABLES: RomDataTables = &[RomDataTable {
    min_revision: 0,
    data_start: 0x4001A18C,
    data_end: 0x4001A318,
    bss_start: 0x4001A318,
    bss_end: 0x4001A418,
}];

pub const ROM_TABLE_ENTRY_SIZE: u32 = 12;

pub const EFUSE_INFO: EfuseInfo = EfuseInfo {
    block0: 0x600B_0800 + 0x2C,
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

pub struct CpuSaveState {
    saved_sysclk_conf_reg: u32,
}

impl CpuSaveState {
    const PCR_SYSCLK_CONF_REG: *mut u32 = 0x6009610c as *mut u32;

    const PCR_SOC_CLK_SEL_M: u32 = 3 << 16;
    const PCR_SOC_CLK_MAX: u32 = 1 << 16;

    pub const fn new() -> Self {
        CpuSaveState {
            saved_sysclk_conf_reg: 0,
        }
    }

    pub fn set_max_cpu_clock(&mut self) {
        self.saved_sysclk_conf_reg = unsafe { Self::PCR_SYSCLK_CONF_REG.read_volatile() };

        unsafe {
            Self::PCR_SYSCLK_CONF_REG.write_volatile(
                (self.saved_sysclk_conf_reg & !Self::PCR_SOC_CLK_SEL_M) | Self::PCR_SOC_CLK_MAX,
            )
        };
    }

    pub fn restore(&self) {
        unsafe { Self::PCR_SYSCLK_CONF_REG.write_volatile(self.saved_sysclk_conf_reg) };
    }
}

pub fn major_chip_version() -> u8 {
    read_field::<1, 117, 2>()
}

pub fn minor_chip_version() -> u8 {
    read_field::<1, 114, 3>()
}

/// Ensures that data (e.g. constants) are accessed through the data bus.
pub unsafe fn read_via_data_bus(s: &u8) -> u8 {
    unsafe { core::ptr::read(s as *const u8) }
}
