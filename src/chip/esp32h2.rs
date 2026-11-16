use crate::{
    efuse::{read_field, EfuseInfo},
    flash::{rom_write_chunk::RomWriteChunk, MemSpi},
    rom::{RomDataTable, RomDataTables},
};

// Max of 16MB
pub const MAX_FLASH_SIZE: u32 = 0x1000000;

pub const ROM_DATA_TABLES: RomDataTables = &[
    RomDataTable {
        min_revision: 0,
        data_start: 0x4001A18C,
        data_end: 0x4001A318,
        bss_start: 0x4001A318,
        bss_end: 0x4001A418,
    },
    // Later ECOs' ROM elfs are not available, let's not initialize memory for them.
    RomDataTable {
        min_revision: 100,
        data_start: 0,
        data_end: 0,
        bss_start: 0,
        bss_end: 0,
    },
];

pub const ROM_TABLE_ENTRY_SIZE: u32 = 12;

pub const EFUSE_INFO: EfuseInfo = EfuseInfo {
    block0: 0x600B_0800 + 0x2C,
    block_sizes: &[6, 6, 8, 8, 8, 8, 8, 8, 8, 8, 8],
};

pub const MEM_SPI: MemSpi = MemSpi {
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

pub struct CpuSaveState {
    saved_sysclk_conf_reg: u32,
    saved_spi1_clock: u32,
    flash_clock_modified: bool,
    write_chunk: RomWriteChunk,
}

impl CpuSaveState {
    const PCR_SYSCLK_CONF_REG: *mut u32 = 0x6009610c as *mut u32;

    const PCR_SOC_CLK_SEL_M: u32 = 3 << 16;
    const PCR_SOC_CLK_MAX: u32 = 1 << 16;

    const PCR_MSPI_CONF_REG: *const u32 = 0x60096018 as *const u32;
    const PCR_MSPI_CLK_SEL_M: u32 = 3 << 3;
    const PCR_MSPI_CLK_SEL_PLL_F64M: u32 = 2 << 3;
    const SPI1_CLOCK: *mut u32 = (MEM_SPI.base + 0x14) as *mut u32;
    // 64 MHz / 2
    const SPI1_CLOCK_DIV2: u32 = (1 << 16) | 1;

    pub const fn new() -> Self {
        CpuSaveState {
            saved_sysclk_conf_reg: 0,
            saved_spi1_clock: 0,
            flash_clock_modified: false,
            write_chunk: RomWriteChunk::new(),
        }
    }

    /// Runs SPI1 at 32 MHz instead of the reset default of 64 MHz / 4, and makes the ROM write
    /// 64 bytes per command.
    pub fn configure_flash(&mut self) {
        let soc_clk_sel =
            unsafe { Self::PCR_SYSCLK_CONF_REG.read_volatile() } & Self::PCR_SOC_CLK_SEL_M;
        let mspi_clk_sel =
            unsafe { Self::PCR_MSPI_CONF_REG.read_volatile() } & Self::PCR_MSPI_CLK_SEL_M;
        if soc_clk_sel == Self::PCR_SOC_CLK_MAX && mspi_clk_sel == Self::PCR_MSPI_CLK_SEL_PLL_F64M {
            unsafe {
                self.saved_spi1_clock = Self::SPI1_CLOCK.read_volatile();
                self.flash_clock_modified = true;
                Self::SPI1_CLOCK.write_volatile(Self::SPI1_CLOCK_DIV2);
            }
        }
        self.write_chunk.enlarge();
    }

    fn restore_flash(&self) {
        self.write_chunk.restore();
        if self.flash_clock_modified {
            unsafe { Self::SPI1_CLOCK.write_volatile(self.saved_spi1_clock) };
        }
    }
    pub fn set_max_cpu_clock(&mut self) {
        self.saved_sysclk_conf_reg = unsafe { Self::PCR_SYSCLK_CONF_REG.read_volatile() };

        unsafe {
            Self::PCR_SYSCLK_CONF_REG.write_volatile(
                (self.saved_sysclk_conf_reg & !Self::PCR_SOC_CLK_SEL_M) | Self::PCR_SOC_CLK_MAX,
            )
        };
    }

    pub fn restore(&self) {
        self.restore_flash();
        unsafe { Self::PCR_SYSCLK_CONF_REG.write_volatile(self.saved_sysclk_conf_reg) };
    }
}

pub fn is_data_ram(address: u32) -> bool {
    (0x4080_0000..0x4085_0000).contains(&address)
}

pub fn major_chip_version() -> u8 {
    read_field::<1, 117, 2>()
}

pub fn minor_chip_version() -> u8 {
    read_field::<1, 114, 3>()
}

/// Ensures that data (e.g. constants) are accessed through the data bus.
pub unsafe fn read_via_data_bus(s: &u8) -> u8 {
    unsafe { core::ptr::read(s as *const u8) }
}
