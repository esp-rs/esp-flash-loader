use crate::{
    efuse::{read_field, EfuseInfo},
    flash::MemSpi,
    rom::{RomDataTable, RomDataTables},
};

// Max of 1GB
pub const MAX_FLASH_SIZE: u32 = 0x40000000;

pub const ROM_DATA_TABLES: RomDataTables = &[RomDataTable {
    min_revision: 0,
    data_start: 0x40057354,
    data_end: 0x400575C4,
    bss_start: 0x400575D4,
    bss_end: 0x400577A8,
}];

pub const ROM_TABLE_ENTRY_SIZE: u32 = 16;

pub const EFUSE_INFO: EfuseInfo = EfuseInfo {
    block0: 0x6000_7000 + 0x2C,
    block_sizes: &[6, 6, 8, 8, 8, 8, 8, 8, 8, 8, 8],
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
}

impl CpuSaveState {
    const SYSTEM_CPU_PER_CONF_REG: *mut u32 = 0x600C0010 as *mut u32;
    const SYSTEM_CPUPERIOD_SEL_M: u32 = 3;
    const SYSTEM_CPUPERIOD_MAX: u32 = 2;

    const SYSTEM_SYSCLK_CONF_REG: *mut u32 = 0x600C0060 as *mut u32;
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

        // FIXME: setting max CPU frequency causes a crash on later chips
        if crate::efuse::read_chip_revision() >= 2 {
            return;
        }

        unsafe {
            Self::SYSTEM_SYSCLK_CONF_REG.write_volatile(
                (self.saved_sysclk_conf_reg & !Self::SYSTEM_SOC_CLK_SEL_M)
                    | Self::SYSTEM_SOC_CLK_MAX,
            )
        };

        // Leave some time for the change to settle
        extern "C" {
            fn ets_delay_us(us: u32);
        }
        unsafe { ets_delay_us(100) };

        unsafe {
            Self::SYSTEM_CPU_PER_CONF_REG.write_volatile(
                (self.saved_cpu_per_conf_reg & !Self::SYSTEM_CPUPERIOD_SEL_M)
                    | Self::SYSTEM_CPUPERIOD_MAX,
            )
        };
    }

    pub fn restore(&self) {
        unsafe { Self::SYSTEM_SYSCLK_CONF_REG.write_volatile(self.saved_sysclk_conf_reg) };
        unsafe { Self::SYSTEM_CPU_PER_CONF_REG.write_volatile(self.saved_cpu_per_conf_reg) };
    }
}

pub fn major_chip_version() -> u8 {
    read_field::<1, 184, 2>()
}

pub fn minor_chip_version() -> u8 {
    let lo = read_field::<1, 114, 3>();
    let hi = read_field::<1, 183, 1>();

    hi << 3 | lo
}

/// Ensures that data (e.g. constants) are accessed through the data bus.
pub unsafe fn read_via_data_bus(s: &u8) -> u8 {
    // SRAM1
    const DBUS_START: usize = 0x3FC8_8000;
    const DBUS_END: usize = 0x3FCF_0000;
    const IBUS_START: usize = 0x4037_8000;

    let addr = s as *const u8 as usize;
    if addr >= DBUS_START && addr < DBUS_END {
        *s
    } else {
        let ptr = addr - IBUS_START + DBUS_START;
        unsafe { core::ptr::read(ptr as *const u8) }
    }
}
