use crate::{
    efuse::{read_field, EfuseInfo},
    rom::{RomDataTable, RomDataTables},
};

pub const STATE_ADDR: usize = 0x3FCB_0000;

// Max of 16MB
pub const MAX_FLASH_SIZE: u32 = 0x1000000;

pub const ROM_DATA_TABLES: RomDataTables = &[
    RomDataTable {
        min_revision: 0,
        data_start: 0x40058898,
        data_end: 0x40058A88,
        bss_start: 0x40058A98,
        bss_end: 0x40058C0C,
    },
    RomDataTable {
        min_revision: 3,
        data_start: 0x40059200,
        data_end: 0x40059400,
        bss_start: 0x40059410,
        bss_end: 0x40059590,
    },
    RomDataTable {
        min_revision: 101,
        data_start: 0x40059620,
        data_end: 0x40059830,
        bss_start: 0x40059840,
        bss_end: 0x400599CC,
    },
];

pub const ROM_TABLE_ENTRY_SIZE: u32 = 16;

pub const EFUSE_INFO: EfuseInfo = EfuseInfo {
    block0: 0x6000_8800 + 0x2C,
    block_sizes: &[6, 6, 8, 8, 8, 8, 8, 8, 8, 8, 8],
};

pub fn major_chip_version() -> u8 {
    read_field::<1, 184, 2>()
}

pub fn minor_chip_version() -> u8 {
    let lo = read_field::<1, 114, 3>();
    let hi = read_field::<1, 183, 1>();

    hi << 3 | lo
}
