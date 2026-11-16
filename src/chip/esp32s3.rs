use crate::{
    efuse::{read_field, EfuseInfo},
    flash::{rom_write_chunk::RomWriteChunk, MemSpi},
    rom::{RomDataTable, RomDataTables},
};

// Max of 1GB
pub const MAX_FLASH_SIZE: u32 = 0x40000000;

pub const ROM_DATA_TABLES: RomDataTables = &[RomDataTable {
    min_revision: 0,
    data_start: 0x40057354,
    data_end: 0x400575C4,
    bss_start: 0x400575D4,
    bss_end: 0x400577A8,
}];

pub const ROM_TABLE_ENTRY_SIZE: u32 = 16;

pub const EFUSE_INFO: EfuseInfo = EfuseInfo {
    block0: 0x6000_7000 + 0x2C,
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

pub struct CpuSaveState {
    saved_cpu_per_conf_reg: u32,
    saved_sysclk_conf_reg: u32,
    saved_rtc_cntl_date_reg: u32,
    modified: bool,
    saved_spi1_clock: u32,
    flash_clock_modified: bool,
    write_chunk: RomWriteChunk,
}

extern "C" {
    fn ets_delay_us(us: u32);
    fn ets_update_cpu_frequency(ticks_per_us: u32);
    fn rom_i2c_writeReg_Mask(block: u32, host_id: u32, reg_add: u32, msb: u32, lsb: u32, data: u32);
    fn ets_efuse_flash_octal_mode() -> bool;
}

impl CpuSaveState {
    // SYSTEM_CPU_PER_CONF_REG (DR_REG_SYSTEM_BASE + 0x10)
    // - CPUPERIOD_SEL at bits[1:0]: 0=80MHz, 1=160MHz, 2=240MHz (from PLL)
    // - PLL_FREQ_SEL  at bit [2]:   0=320MHz PLL,  1=480MHz PLL (default = 1)
    const SYSTEM_CPU_PER_CONF_REG: *mut u32 = 0x600C0010 as *mut u32;
    const SYSTEM_CPUPERIOD_SEL_M: u32 = 0b11;
    const SYSTEM_PLL_FREQ_SEL_M: u32 = 1 << 2;
    /// Value to write for CPUPERIOD_SEL=2 (240 MHz) and PLL_FREQ_SEL=1 (480 MHz PLL).
    const SYSTEM_CPU_PER_CONF_240M: u32 = (2 << 0) | (1 << 2);

    // SYSTEM_SYSCLK_CONF_REG (DR_REG_SYSTEM_BASE + 0x60)
    // - SOC_CLK_SEL at bits[11:10]: 0=XTAL, 1=PLL, 2=RC_FAST
    const SYSTEM_SYSCLK_CONF_REG: *mut u32 = 0x600C0060 as *mut u32;
    const SYSTEM_SOC_CLK_SEL_M: u32 = 3 << 10;
    const SYSTEM_SOC_CLK_PLL: u32 = 1 << 10;

    // RTC_CNTL_DATE_REG (DR_REG_RTCCNTL_BASE + 0x1FC)
    // - SLAVE_PD at bits[18:13]: controls the 6 LDO slaves used to track the
    //   CPU clock; value is `0x7 >> (cpu_freq_mhz / 80)`.
    const RTC_CNTL_DATE_REG: *mut u32 = 0x600081FC as *mut u32;
    const RTC_CNTL_SLAVE_PD_M: u32 = 0x3F << 13;

    // Digital regulator (LDO) controls exposed through the on-chip I2C bus.
    // Raising the RTC/DIG dbias values is required before switching the CPU
    // to 240 MHz, otherwise later ECO revisions lock up. The target dbias
    // value of 28 matches esp-hal's non-PVT default for the 240 MHz preset.
    const I2C_DIG_REG: u32 = 0x6D;
    const I2C_DIG_REG_HOSTID: u32 = 1;
    const I2C_DIG_REG_EXT_RTC_DREG: u32 = 4;
    const I2C_DIG_REG_EXT_DIG_DREG: u32 = 6;
    const I2C_DIG_DREG_MSB: u32 = 4;
    const I2C_DIG_DREG_LSB: u32 = 0;
    const DBIAS_240M: u32 = 28;

    const SPI1_CLOCK: *mut u32 = (MEM_SPI.base + 0x14) as *mut u32;
    const SPI_MEM_CORE_CLK_SEL_REG: *const u32 = 0x6000_30EC as *const u32;
    const CORE_CLK_SEL_M: u32 = 3;
    // 80 MHz / 2
    const SPI1_CLOCK_DIV2: u32 = (1 << 16) | 1;

    pub const fn new() -> Self {
        CpuSaveState {
            saved_cpu_per_conf_reg: 0,
            saved_sysclk_conf_reg: 0,
            saved_rtc_cntl_date_reg: 0,
            modified: false,
            saved_spi1_clock: 0,
            flash_clock_modified: false,
            write_chunk: RomWriteChunk::new(),
        }
    }

    /// Runs SPI1 at 40 MHz instead of the reset default of 80 MHz / 4, and makes the ROM write
    /// 64 bytes per command.
    pub fn configure_flash(&mut self) {
        let soc_clk_sel =
            unsafe { Self::SYSTEM_SYSCLK_CONF_REG.read_volatile() } & Self::SYSTEM_SOC_CLK_SEL_M;
        let core_clk_sel =
            unsafe { Self::SPI_MEM_CORE_CLK_SEL_REG.read_volatile() } & Self::CORE_CLK_SEL_M;
        // The SPI0/1 core clock is 80 MHz only with the CPU on the PLL.
        if soc_clk_sel == Self::SYSTEM_SOC_CLK_PLL && core_clk_sel == 0 {
            unsafe {
                self.saved_spi1_clock = Self::SPI1_CLOCK.read_volatile();
                self.flash_clock_modified = true;
                Self::SPI1_CLOCK.write_volatile(Self::SPI1_CLOCK_DIV2);
            }
        }
        // Octal flash uses the loader's own function table.
        if !unsafe { ets_efuse_flash_octal_mode() } {
            self.write_chunk.enlarge();
        }
    }

    fn restore_flash(&self) {
        self.write_chunk.restore();
        if self.flash_clock_modified {
            unsafe { Self::SPI1_CLOCK.write_volatile(self.saved_spi1_clock) };
        }
    }

    pub fn set_max_cpu_clock(&mut self) {
        self.saved_cpu_per_conf_reg = unsafe { Self::SYSTEM_CPU_PER_CONF_REG.read_volatile() };
        self.saved_sysclk_conf_reg = unsafe { Self::SYSTEM_SYSCLK_CONF_REG.read_volatile() };
        self.saved_rtc_cntl_date_reg = unsafe { Self::RTC_CNTL_DATE_REG.read_volatile() };
        self.modified = true;

        // Raise RTC and digital LDO bias voltages before switching to 240 MHz.
        // Without this, ECO revisions >= 2 hang when CPUPERIOD_SEL is set to 2.
        unsafe {
            rom_i2c_writeReg_Mask(
                Self::I2C_DIG_REG,
                Self::I2C_DIG_REG_HOSTID,
                Self::I2C_DIG_REG_EXT_RTC_DREG,
                Self::I2C_DIG_DREG_MSB,
                Self::I2C_DIG_DREG_LSB,
                Self::DBIAS_240M,
            );
            rom_i2c_writeReg_Mask(
                Self::I2C_DIG_REG,
                Self::I2C_DIG_REG_HOSTID,
                Self::I2C_DIG_REG_EXT_DIG_DREG,
                Self::I2C_DIG_DREG_MSB,
                Self::I2C_DIG_DREG_LSB,
                Self::DBIAS_240M,
            );
            ets_delay_us(40);
        }

        // Update the LDO slave mask for CPU_FREQ = 240 MHz: pd_slave = 3,
        // so SLAVE_PD = 0x7 >> 3 = 0 (all 6 slaves active).
        unsafe {
            Self::RTC_CNTL_DATE_REG
                .write_volatile(self.saved_rtc_cntl_date_reg & !Self::RTC_CNTL_SLAVE_PD_M);
        }

        // Program 240 MHz CPU from the 480 MHz PLL.
        unsafe {
            Self::SYSTEM_CPU_PER_CONF_REG.write_volatile(
                (self.saved_cpu_per_conf_reg
                    & !(Self::SYSTEM_CPUPERIOD_SEL_M | Self::SYSTEM_PLL_FREQ_SEL_M))
                    | Self::SYSTEM_CPU_PER_CONF_240M,
            );
        }

        // Switch the SoC clock source to PLL.
        unsafe {
            Self::SYSTEM_SYSCLK_CONF_REG.write_volatile(
                (self.saved_sysclk_conf_reg & !Self::SYSTEM_SOC_CLK_SEL_M)
                    | Self::SYSTEM_SOC_CLK_PLL,
            );
        }

        // Keep the ROM's cached CPU tick-per-us value in sync so any later
        // ROM call that relies on ets_delay_us continues to produce accurate
        // timings.
        unsafe { ets_update_cpu_frequency(240) };
    }

    pub fn restore(&self) {
        self.restore_flash();
        if !self.modified {
            return;
        }

        unsafe {
            Self::SYSTEM_SYSCLK_CONF_REG.write_volatile(self.saved_sysclk_conf_reg);
            Self::SYSTEM_CPU_PER_CONF_REG.write_volatile(self.saved_cpu_per_conf_reg);
            Self::RTC_CNTL_DATE_REG.write_volatile(self.saved_rtc_cntl_date_reg);

            let previous_mhz =
                previous_cpu_mhz(self.saved_cpu_per_conf_reg, self.saved_sysclk_conf_reg);
            ets_update_cpu_frequency(previous_mhz);
        }
    }
}

pub fn is_data_ram(address: u32) -> bool {
    (0x3FC8_8000..0x3FD0_0000).contains(&address)
}

fn previous_cpu_mhz(cpu_per_conf: u32, sysclk_conf: u32) -> u32 {
    // Mirror of `clk_ll_cpu_get_freq_mhz_from_*` in esp-idf, used only to
    // restore the ROM's cached tick rate when UnInit is called.
    let soc_clk_sel = (sysclk_conf >> 10) & 0b11;
    match soc_clk_sel {
        // XTAL
        0 => 40,
        // PLL
        1 => {
            let cpuperiod = cpu_per_conf & 0b11;
            let pll_is_480m = ((cpu_per_conf >> 2) & 1) != 0;
            match (cpuperiod, pll_is_480m) {
                (0, _) => 80,
                (1, _) => 160,
                (2, true) => 240,
                _ => 80,
            }
        }
        // RC_FAST (~17.5 MHz); round to 20 MHz which matches esp-idf's default.
        2 => 20,
        _ => 40,
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
    // SRAM1
    const DBUS_START: usize = 0x3FC8_8000;
    const DBUS_END: usize = 0x3FCF_0000;
    const IBUS_START: usize = 0x4037_8000;

    let addr = s as *const u8 as usize;
    if addr >= DBUS_START && addr < DBUS_END {
        *s
    } else {
        let ptr = addr - IBUS_START + DBUS_START;
        unsafe { core::ptr::read(ptr as *const u8) }
    }
}
