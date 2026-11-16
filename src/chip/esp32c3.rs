use crate::{
    efuse::{read_field, EfuseInfo},
    flash::MemSpi,
    rom::{RomDataTable, RomDataTables},
};

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

pub const MEM_SPI: MemSpi = MemSpi {
    base: 0x6000_2000,
    cmd: 0x00,
    addr: 0x04,
    ctrl: 0x08,
    user: 0x18,
    user1: 0x1C,
    user2: 0x20,
    miso_dlen: 0x28,
    data_buf_0: 0x58,
};

extern "C" {
    static mut rom_spiflash_legacy_funcs: u32;
}

pub struct CpuSaveState {
    saved_cpu_per_conf_reg: u32,
    saved_sysclk_conf_reg: u32,
    saved_spi1_clock: u32,
    flash_clock_modified: bool,
    saved_legacy_funcs: u32,
    legacy_funcs: [u32; 11],
}

impl CpuSaveState {
    const SYSTEM_CPU_PER_CONF_REG: *mut u32 = 0x600C0008 as *mut u32;
    const SYSTEM_CPUPERIOD_SEL_M: u32 = 3;
    const SYSTEM_CPUPERIOD_MAX: u32 = 1;

    const SYSTEM_SYSCLK_CONF_REG: *mut u32 = 0x600C0058 as *mut u32;
    const SYSTEM_SOC_CLK_SEL_M: u32 = 3 << 10;
    const SYSTEM_SOC_CLK_MAX: u32 = 1 << 10;

    const SPI1_CLOCK: *mut u32 = (MEM_SPI.base + 0x14) as *mut u32;
    const SPI_MEM_CORE_CLK_SEL_REG: *const u32 = 0x6000_30E0 as *const u32;
    const SPI01_CLK_SEL_M: u32 = 3;
    // 80 MHz / 2
    const SPI1_CLOCK_DIV2: u32 = (1 << 16) | 1;

    pub const fn new() -> Self {
        CpuSaveState {
            saved_cpu_per_conf_reg: 0,
            saved_sysclk_conf_reg: 0,
            saved_spi1_clock: 0,
            flash_clock_modified: false,
            saved_legacy_funcs: 0,
            legacy_funcs: [0; 11],
        }
    }

    /// Runs SPI1 at 40 MHz instead of the reset default of 80 MHz / 4, and makes the ROM write
    /// 64 bytes per command.
    pub fn configure_flash(&mut self) {
        let soc_clk_sel =
            unsafe { Self::SYSTEM_SYSCLK_CONF_REG.read_volatile() } & Self::SYSTEM_SOC_CLK_SEL_M;
        let spi01_clk_sel =
            unsafe { Self::SPI_MEM_CORE_CLK_SEL_REG.read_volatile() } & Self::SPI01_CLK_SEL_M;
        // The SPI0/1 core clock is 80 MHz only with the CPU on the PLL.
        if soc_clk_sel == Self::SYSTEM_SOC_CLK_MAX && spi01_clk_sel == 0 {
            unsafe {
                self.saved_spi1_clock = Self::SPI1_CLOCK.read_volatile();
                self.flash_clock_modified = true;
                Self::SPI1_CLOCK.write_volatile(Self::SPI1_CLOCK_DIV2);
            }
        }
        self.enlarge_write_chunk();
    }

    /// The ROM's function table is in ROM, so this points the ROM at a copy with
    /// `write_sub_len` set to 64.
    fn enlarge_write_chunk(&mut self) {
        const READ_SUB_LEN: usize = 0;
        const WRITE_SUB_LEN: usize = 1;

        let funcs = unsafe { rom_spiflash_legacy_funcs };
        if funcs == 0 || funcs & 3 != 0 {
            return;
        }
        for (i, word) in self.legacy_funcs.iter_mut().enumerate() {
            *word = unsafe { (funcs as *const u32).add(i).read_volatile() };
        }
        if self.legacy_funcs[READ_SUB_LEN] != 16 || self.legacy_funcs[WRITE_SUB_LEN] != 32 {
            return;
        }
        self.legacy_funcs[WRITE_SUB_LEN] = 64;
        self.saved_legacy_funcs = funcs;
        unsafe { rom_spiflash_legacy_funcs = self.legacy_funcs.as_ptr() as u32 };
    }

    fn restore_flash(&self) {
        if self.saved_legacy_funcs != 0 {
            unsafe { rom_spiflash_legacy_funcs = self.saved_legacy_funcs };
        }
        if self.flash_clock_modified {
            unsafe { Self::SPI1_CLOCK.write_volatile(self.saved_spi1_clock) };
        }
    }

    pub fn set_max_cpu_clock(&mut self) {
        self.saved_cpu_per_conf_reg = unsafe { Self::SYSTEM_CPU_PER_CONF_REG.read_volatile() };
        self.saved_sysclk_conf_reg = unsafe { Self::SYSTEM_SYSCLK_CONF_REG.read_volatile() };

        unsafe {
            Self::SYSTEM_CPU_PER_CONF_REG.write_volatile(
                (self.saved_cpu_per_conf_reg & !Self::SYSTEM_CPUPERIOD_SEL_M)
                    | Self::SYSTEM_CPUPERIOD_MAX,
            );
            Self::SYSTEM_SYSCLK_CONF_REG.write_volatile(
                (self.saved_sysclk_conf_reg & !Self::SYSTEM_SOC_CLK_SEL_M)
                    | Self::SYSTEM_SOC_CLK_MAX,
            );
        }
    }

    pub fn restore(&self) {
        self.restore_flash();
        unsafe { Self::SYSTEM_SYSCLK_CONF_REG.write_volatile(self.saved_sysclk_conf_reg) };
        unsafe { Self::SYSTEM_CPU_PER_CONF_REG.write_volatile(self.saved_cpu_per_conf_reg) };
    }
}

pub fn major_chip_version() -> u8 {
    read_field::<1, 184, 2>()
}

pub fn minor_chip_version() -> u8 {
    let lo = read_field::<1, 114, 3>();
    let hi = read_field::<1, 183, 1>();

    hi << 3 | lo
}

/// Ensures that data (e.g. constants) are accessed through the data bus.
pub unsafe fn read_via_data_bus(s: &u8) -> u8 {
    unsafe { core::ptr::read(s as *const u8) }
}
