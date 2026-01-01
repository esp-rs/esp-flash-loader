use crate::{
    efuse::{read_field, EfuseInfo},
    flash::MemSpi,
    rom::{RomDataTable, RomDataTables},
};

// Max of 64MB
pub const MAX_FLASH_SIZE: u32 = 0x4000000;

pub const ROM_DATA_TABLES: RomDataTables = &[
    RomDataTable {
        // ECO0
        min_revision: 0,
        data_start: 0x4FC1BEC8,
        data_end: 0x4FC1C054,
        bss_start: 0x4FC1C054,
        bss_end: 0x4FC1C154,
    },
    RomDataTable {
        // ECO1
        min_revision: 1,
        data_start: 0x4FC1BF9C,
        data_end: 0x4FC1C128,
        bss_start: 0x4FC1C128,
        bss_end: 0x4FC1C228,
    },
    RomDataTable {
        // ECO2-ECO4
        min_revision: 100,
        data_start: 0x4FC1C4A8,
        data_end: 0x4FC1C634,
        bss_start: 0x4FC1C634,
        bss_end: 0x4FC1C734,
    },
    RomDataTable {
        // ECO5
        min_revision: 300,
        data_start: 0x4FC1C860,
        data_end: 0x4FC1C9EC,
        bss_start: 0x4FC1C9EC,
        bss_end: 0x4FC1CAEC,
    },
];

pub const ROM_TABLE_ENTRY_SIZE: u32 = 12;

pub const EFUSE_INFO: EfuseInfo = EfuseInfo {
    block0: 0x5012_D000 + 0x2C,
    block_sizes: &[6, 6, 8, 8, 8, 8, 8, 8, 8, 8, 8],
};

pub const MEM_SPI: MemSpi = MemSpi {
    base: 0x5008_D000,
    cmd: 0x00,
    addr: 0x04,
    ctrl: 0x08,
    user: 0x18,
    user1: 0x1C,
    user2: 0x20,
    miso_dlen: 0x28,
    data_buf_0: 0x58,
};

pub struct CpuSaveState {}

impl CpuSaveState {
    pub const fn new() -> Self {
        CpuSaveState {}
    }

    pub fn set_max_cpu_clock(&mut self) {}

    pub fn restore(&self) {}
}

pub fn major_chip_version() -> u8 {
    let lo = read_field::<1, 68, 2>();
    let hi = read_field::<1, 87, 1>();
    hi << 2 | lo
}

pub fn minor_chip_version() -> u8 {
    read_field::<1, 64, 4>()
}

/// Ensures that data (e.g. constants) are accessed through the data bus.
pub unsafe fn read_via_data_bus(s: &u8) -> u8 {
    unsafe { core::ptr::read(s as *const u8) }
}
