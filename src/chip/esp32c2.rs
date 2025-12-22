use crate::{
    efuse::{read_field, EfuseInfo},
    rom::{RomDataTable, RomDataTables},
};

// Max of 16MB
pub const MAX_FLASH_SIZE: u32 = 0x1000000;

pub const ROM_DATA_TABLES: RomDataTables = &[
    RomDataTable {
        min_revision: 0,
        data_start: 0x40082174,
        data_end: 0x400823F4,
        bss_start: 0x40082404,
        bss_end: 0x400825E4,
    },
    RomDataTable {
        min_revision: 400,
        data_start: 0x40081EEC,
        data_end: 0x4008219C,
        bss_start: 0x400821AC,
        bss_end: 0x400823B0,
    },
];

pub const ROM_TABLE_ENTRY_SIZE: u32 = 16;

pub const EFUSE_INFO: EfuseInfo = EfuseInfo {
    block0: 0x6000_8800 + 0x2C,
    block_sizes: &[8, 12, 32, 32],
};

pub fn major_chip_version() -> u8 {
    read_field::<2, 52, 2>()
}

pub fn minor_chip_version() -> u8 {
    read_field::<2, 48, 4>()
}
