use crate::{
    efuse::{read_field, EfuseInfo},
    flash::MemSpi,
    rom::RomDataTables,
};

// Max of 16MB (SOC_MMU_MAX_PADDR_PAGE_NUM * 64KB, IROM window 0x42000000..0x43000000)
pub const MAX_FLASH_SIZE: u32 = 0x1000000;

// TODO: fill from H21 ROM ELF when available.
pub const ROM_DATA_TABLES: RomDataTables = &[];

pub const ROM_TABLE_ENTRY_SIZE: u32 = 12;

pub const EFUSE_INFO: EfuseInfo = EfuseInfo {
    // EFUSE peripheral base = 0x600B4000; readable data starts at offset 0x2C
    block0: 0x600B_4000 + 0x2C,
    block_sizes: &[6, 6, 8, 8, 8, 8, 8, 8, 8, 8, 8],
};

pub const MEM_SPI: MemSpi = MemSpi {
    // SPIMEM1 (SPI1 flash controller) base address
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

pub struct CpuSaveState {}

impl CpuSaveState {
    pub const fn new() -> Self {
        CpuSaveState {}
    }

    pub fn set_max_cpu_clock(&mut self) {}

    pub fn restore(&self) {}
}

// EFUSE_BLK1, bit 168, 2 bits (WAFER_VERSION_MAJOR)
pub fn major_chip_version() -> u8 {
    read_field::<1, 168, 2>()
}

// EFUSE_BLK1, bit 164, 4 bits (WAFER_VERSION_MINOR)
pub fn minor_chip_version() -> u8 {
    read_field::<1, 164, 4>()
}

/// Ensures that data (e.g. constants) are accessed through the data bus.
pub unsafe fn read_via_data_bus(s: &u8) -> u8 {
    unsafe { core::ptr::read(s as *const u8) }
}
