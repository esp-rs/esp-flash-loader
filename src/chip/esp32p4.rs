use crate::{
    efuse::{read_field, EfuseInfo},
    rom::{RomDataTable, RomDataTables},
};

pub const STATE_ADDR: usize = 0x8FF6_0000;

// Max of 64MB
pub const MAX_FLASH_SIZE: u32 = 0x4000000;

pub const ROM_DATA_TABLES: RomDataTables = &[
    RomDataTable {
        min_revision: 100,
        data_start: 0x4FC1BEC8,
        data_end: 0x4FC1C054,
        bss_start: 0x4FC1C054,
        bss_end: 0x4FC1C154,
    },
    RomDataTable {
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

pub fn major_chip_version() -> u8 {
    let lo = read_field::<1, 68, 2>();
    let hi = read_field::<1, 87, 1>();
    hi << 2 | lo
}

pub fn minor_chip_version() -> u8 {
    read_field::<1, 64, 4>()
}
