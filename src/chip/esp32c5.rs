use crate::{
    efuse::{read_field, EfuseInfo},
    rom::{RomDataTable, RomDataTables},
};

// Max of 32MB
pub const MAX_FLASH_SIZE: u32 = 0x2000000;

pub const ROM_DATA_TABLES: RomDataTables = &[
    RomDataTable {
        min_revision: 0,
        data_start: 0x400478A8,
        data_end: 0x40047A7C,
        bss_start: 0x40047A7C,
        bss_end: 0x40047BAC,
    },
    RomDataTable {
        min_revision: 100,
        data_start: 0x4003B154,
        data_end: 0x4003B340,
        bss_start: 0x4003B340,
        bss_end: 0x4003B480,
    },
];

pub const ROM_TABLE_ENTRY_SIZE: u32 = 12;

pub const EFUSE_INFO: EfuseInfo = EfuseInfo {
    block0: 0x600B_4800 + 0x2C,
    block_sizes: &[6, 6, 8, 8, 8, 8, 8, 8, 8, 8, 8],
};

pub fn major_chip_version() -> u8 {
    read_field::<1, 68, 2>()
}

pub fn minor_chip_version() -> u8 {
    read_field::<1, 64, 4>()
}
