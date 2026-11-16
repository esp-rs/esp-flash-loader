use crate::{
    efuse::{read_field, EfuseInfo},
    flash::{rom_write_chunk::RomWriteChunk, MemSpi},
    rom::{RomDataTable, RomDataTables},
};

// Max of 16MB
pub const MAX_FLASH_SIZE: u32 = 0x1000000;

pub const ROM_DATA_TABLES: RomDataTables = &[RomDataTable {
    min_revision: 0,
    data_start: 0x40041EA8,
    data_end: 0x40042064,
    bss_start: 0x40042064,
    bss_end: 0x40042184,
}];

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
    const PCR_SYSCLK_CONF_REG: *mut u32 = 0x60096110 as *mut u32;
    const PCR_MSPI_CLK_CONF_REG: *const u32 = 0x6009601C as *const u32;
    const SPI1_CLOCK: *mut u32 = (MEM_SPI.base + 0x14) as *mut u32;

    const PCR_SOC_CLK_SEL_M: u32 = 3 << 16;
    const PCR_SOC_CLK_MAX: u32 = 1 << 16;
    const MSPI_FAST_HS_DIV_NUM_SHIFT: u32 = 8;
    const SPI1_MAX_MHZ: u32 = 40;

    pub const fn new() -> Self {
        CpuSaveState {
            saved_sysclk_conf_reg: 0,
            saved_spi1_clock: 0,
            flash_clock_modified: false,
            write_chunk: RomWriteChunk::new(),
        }
    }

    /// Runs SPI1 at up to 40 MHz instead of the reset default MSPI_FAST_CLK / 4, and makes the
    /// ROM write 64 bytes per command.
    pub fn configure_flash(&mut self) {
        let soc_clk_sel =
            unsafe { Self::PCR_SYSCLK_CONF_REG.read_volatile() } & Self::PCR_SOC_CLK_SEL_M;
        let hs_div_num = unsafe { Self::PCR_MSPI_CLK_CONF_REG.read_volatile() }
            >> Self::MSPI_FAST_HS_DIV_NUM_SHIFT
            & 0xFF;

        // With the CPU on the PLL, MSPI_FAST_CLK is PLL_F480M divided by 4, 5 or 6.
        if soc_clk_sel == Self::PCR_SOC_CLK_MAX && (3..=5).contains(&hs_div_num) {
            let mspi_mhz = 480 / (hs_div_num + 1);
            let div = mspi_mhz.div_ceil(Self::SPI1_MAX_MHZ);
            unsafe {
                self.saved_spi1_clock = Self::SPI1_CLOCK.read_volatile();
                self.flash_clock_modified = true;
                // CLKCNT_N, CLKCNT_H and CLKCNT_L
                Self::SPI1_CLOCK
                    .write_volatile(((div - 1) << 16) | ((div / 2 - 1) << 8) | (div - 1));
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
    (0x4080_0000..0x4088_0000).contains(&address)
}

pub fn major_chip_version() -> u8 {
    read_field::<1, 118, 2>()
}

pub fn minor_chip_version() -> u8 {
    read_field::<1, 114, 4>()
}

/// Ensures that data (e.g. constants) are accessed through the data bus.
pub unsafe fn read_via_data_bus(s: &u8) -> u8 {
    unsafe { core::ptr::read(s as *const u8) }
}
