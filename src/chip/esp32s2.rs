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

pub fn major_chip_version() -> u8 {
    read_field::<1, 114, 2>()
}

pub fn minor_chip_version() -> u8 {
    let lo = read_field::<1, 132, 3>();
    let hi = read_field::<1, 116, 1>();

    hi << 3 | lo
}
