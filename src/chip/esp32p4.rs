use crate::{
    efuse::{read_field, EfuseInfo},
    flash::{rom_write_chunk::RomWriteChunk, MemSpi},
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
    RomDataTable {
        // ECO6
        min_revision: 301,
        data_start: 0x4FC1C8F8,
        data_end: 0x4FC1CA84,
        bss_start: 0x4FC1CA84,
        bss_end: 0x4FC1CB84,
    },
    RomDataTable {
        // ECO7
        min_revision: 302,
        data_start: 0x4FC1C9D8,
        data_end: 0x4FC1CB64,
        bss_start: 0x4FC1CB64,
        bss_end: 0x4FC1CC64,
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

pub struct CpuSaveState {
    saved_peri_clk_ctrl00: u32,
    saved_spi1_clock: u32,
    flash_clock_modified: bool,
    write_chunk: RomWriteChunk,
}

impl CpuSaveState {
    const PERI_CLK_CTRL00: *mut u32 = 0x500E_6030 as *mut u32;
    const SPI1_CLOCK: *mut u32 = (MEM_SPI.base + 0x14) as *mut u32;

    // FLASH_CLK_SRC_SEL and FLASH_CORE_CLK_DIV_NUM. Zero selects XTAL_CLK, undivided.
    const FLASH_CLK_SRC_MASK: u32 = 0xFF3;
    const SPI1_CLK_EQU_SYSCLK: u32 = 1 << 31;

    pub const fn new() -> Self {
        CpuSaveState {
            saved_peri_clk_ctrl00: 0,
            saved_spi1_clock: 0,
            flash_clock_modified: false,
            write_chunk: RomWriteChunk::new(),
        }
    }

    pub fn set_max_cpu_clock(&mut self) {}

    /// Runs SPI1 at the 40 MHz XTAL_CLK instead of the reset default XTAL_CLK / 4, and makes
    /// the ROM write 64 bytes per command.
    pub fn configure_flash(&mut self) {
        unsafe {
            self.saved_peri_clk_ctrl00 = Self::PERI_CLK_CTRL00.read_volatile();
            self.saved_spi1_clock = Self::SPI1_CLOCK.read_volatile();
            self.flash_clock_modified = true;

            Self::PERI_CLK_CTRL00
                .write_volatile(self.saved_peri_clk_ctrl00 & !Self::FLASH_CLK_SRC_MASK);
            Self::SPI1_CLOCK.write_volatile(Self::SPI1_CLK_EQU_SYSCLK);
        }
        if major_chip_version() >= 3 {
            self.write_chunk.enlarge_at(V3_ROM_SPIFLASH_LEGACY_FUNCS);
        } else {
            self.write_chunk.enlarge();
        }
    }

    pub fn restore(&self) {
        self.write_chunk.restore();
        if !self.flash_clock_modified {
            return;
        }
        unsafe {
            Self::SPI1_CLOCK.write_volatile(self.saved_spi1_clock);
            Self::PERI_CLK_CTRL00.write_volatile(self.saved_peri_clk_ctrl00);
        }
    }
}

// `rom_spiflash_legacy_funcs` in the v3 ROM. The linker script has the address in older ROMs.
const V3_ROM_SPIFLASH_LEGACY_FUNCS: u32 = 0x4FFB_FFEC;

pub fn is_data_ram(address: u32) -> bool {
    (0x4FF0_0000..0x4FFC_0000).contains(&address)
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
