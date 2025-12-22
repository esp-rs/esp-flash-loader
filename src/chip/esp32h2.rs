use crate::{
    efuse::{read_field, EfuseInfo},
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

pub fn major_chip_version() -> u8 {
    read_field::<1, 117, 2>()
}

pub fn minor_chip_version() -> u8 {
    read_field::<1, 114, 3>()
}
