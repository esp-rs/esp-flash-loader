use crate::{
    efuse::{read_field, EfuseInfo},
    rom::{RomDataTable, RomDataTables},
};

pub const STATE_ADDR: usize = 0x4082_0000;

// Max of 32MB
pub const MAX_FLASH_SIZE: u32 = 0x2000000;

pub const ROM_DATA_TABLES: RomDataTables = &[
    RomDataTable {
        min_revision: 0,
        data_start: 0x4003700C,
        data_end: 0x400371E0,
        bss_start: 0x400371E0,
        bss_end: 0x40037310,
    },
    RomDataTable {
        min_revision: 100,
        data_start: 0x40038A4C,
        data_end: 0x40038C38,
        bss_start: 0x40038C38,
        bss_end: 0x40038D78,
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
