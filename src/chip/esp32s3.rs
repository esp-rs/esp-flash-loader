use crate::{
    efuse::{read_field, EfuseInfo},
    flash::MemSpi,
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
}

extern "C" {
    fn ets_delay_us(us: u32);
    fn ets_update_cpu_frequency(ticks_per_us: u32);
    fn rom_i2c_writeReg(block: u32, host_id: u32, reg_add: u32, data: u32);
    fn rom_i2c_writeReg_Mask(block: u32, host_id: u32, reg_add: u32, msb: u32, lsb: u32, data: u32);
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
    // - PRE_DIV_CNT at bits[9:0]
    // - SOC_CLK_SEL at bits[11:10]: 0=XTAL, 1=PLL, 2=RC_FAST
    const SYSTEM_SYSCLK_CONF_REG: *mut u32 = 0x600C0060 as *mut u32;
    const SYSTEM_PRE_DIV_CNT_M: u32 = 0x3FF;
    const SYSTEM_SOC_CLK_SEL_M: u32 = 3 << 10;
    const SYSTEM_SOC_CLK_PLL: u32 = 1 << 10;

    const RTC_CNTL_OPTIONS0_REG: *mut u32 = 0x60008000 as *mut u32;
    const RTC_CNTL_BB_I2C_FORCE_PD: u32 = 1 << 6;
    const RTC_CNTL_BBPLL_I2C_FORCE_PD: u32 = 1 << 8;
    const RTC_CNTL_BBPLL_FORCE_PD: u32 = 1 << 10;
    const RTC_CNTL_BBPLL_PD_M: u32 = Self::RTC_CNTL_BB_I2C_FORCE_PD
        | Self::RTC_CNTL_BBPLL_I2C_FORCE_PD
        | Self::RTC_CNTL_BBPLL_FORCE_PD;

    // RTC_CNTL_DATE_REG (DR_REG_RTCCNTL_BASE + 0x1FC)
    // - SLAVE_PD at bits[18:13]: controls the 6 LDO slaves used to track the
    //   CPU clock; value is `0x7 >> (cpu_freq_mhz / 80)`.
    const RTC_CNTL_DATE_REG: *mut u32 = 0x600081FC as *mut u32;
    const RTC_CNTL_SLAVE_PD_M: u32 = 0x3F << 13;

    const I2C_MST_ANA_CONF0_REG: *mut u32 = 0x6000E040 as *mut u32;
    const I2C_MST_BBPLL_STOP_FORCE_HIGH: u32 = 1 << 2;
    const I2C_MST_BBPLL_STOP_FORCE_LOW: u32 = 1 << 3;
    const I2C_MST_BBPLL_CAL_DONE: u32 = 1 << 24;

    const ANA_CONFIG_REG: *mut u32 = 0x6000E044 as *mut u32;
    /// Clear to enable the analog I2C bus to the BBPLL.
    const I2C_BBPLL_M: u32 = 1 << 17;

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

    const I2C_BBPLL: u32 = 0x66;
    const I2C_BBPLL_HOSTID: u32 = 1;
    const I2C_BBPLL_OC_REF: u32 = 2;
    const I2C_BBPLL_OC_DIV_REG: u32 = 3;
    const I2C_BBPLL_REG4: u32 = 4;
    const I2C_BBPLL_OC_DR: u32 = 5;
    const I2C_BBPLL_REG6: u32 = 6;
    const I2C_BBPLL_REG9: u32 = 9;
    /// 480 MHz PLL, MODE_HF set. Do not use 0x69 (320 MHz); that breaks USB-Serial-JTAG.
    const I2C_BBPLL_MODE_HF_480M: u32 = 0x6B;

    pub const fn new() -> Self {
        CpuSaveState {
            saved_cpu_per_conf_reg: 0,
            saved_sysclk_conf_reg: 0,
            saved_rtc_cntl_date_reg: 0,
            modified: false,
        }
    }

    pub fn set_max_cpu_clock(&mut self) {
        self.saved_cpu_per_conf_reg = unsafe { Self::SYSTEM_CPU_PER_CONF_REG.read_volatile() };
        self.saved_sysclk_conf_reg = unsafe { Self::SYSTEM_SYSCLK_CONF_REG.read_volatile() };
        self.saved_rtc_cntl_date_reg = unsafe { Self::RTC_CNTL_DATE_REG.read_volatile() };
        self.modified = true;

        Self::raise_voltage_for_240m();
        Self::enable_pll_clk();

        unsafe {
            Self::SYSTEM_CPU_PER_CONF_REG.write_volatile(
                (Self::SYSTEM_CPU_PER_CONF_REG.read_volatile()
                    & !(Self::SYSTEM_CPUPERIOD_SEL_M | Self::SYSTEM_PLL_FREQ_SEL_M))
                    | Self::SYSTEM_CPU_PER_CONF_240M,
            );
        }

        // Switch the SoC clock source to PLL. Clear PRE_DIV_CNT; it only
        // divides XTAL/RC_FAST, but esp-hal zeros it at the same write.
        unsafe {
            Self::SYSTEM_SYSCLK_CONF_REG.write_volatile(
                (self.saved_sysclk_conf_reg
                    & !(Self::SYSTEM_SOC_CLK_SEL_M | Self::SYSTEM_PRE_DIV_CNT_M))
                    | Self::SYSTEM_SOC_CLK_PLL,
            );
        }

        // Keep the ROM's cached CPU tick-per-us value in sync so any later
        // ROM call that relies on ets_delay_us continues to produce accurate
        // timings.
        unsafe {
            ets_update_cpu_frequency(240);
            // ROM SPI attach after SOC_CLK_SEL=PLL needs the new clock to settle.
            ets_delay_us(100);
        }
    }

    fn raise_voltage_for_240m() {
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

            // LDO slaves: 240 MHz uses all 6, so SLAVE_PD = 0x7 >> 3 = 0.
            Self::RTC_CNTL_DATE_REG.write_volatile(
                Self::RTC_CNTL_DATE_REG.read_volatile() & !Self::RTC_CNTL_SLAVE_PD_M,
            );
        }
    }

    fn bbpll_cal_done() -> bool {
        unsafe { Self::I2C_MST_ANA_CONF0_REG.read_volatile() & Self::I2C_MST_BBPLL_CAL_DONE != 0 }
    }

    fn bbpll_calibration_start() {
        unsafe {
            let mut conf0 = Self::I2C_MST_ANA_CONF0_REG.read_volatile();
            conf0 &= !Self::I2C_MST_BBPLL_STOP_FORCE_HIGH;
            conf0 |= Self::I2C_MST_BBPLL_STOP_FORCE_LOW;
            Self::I2C_MST_ANA_CONF0_REG.write_volatile(conf0);
        }
    }

    fn bbpll_calibration_stop() {
        unsafe {
            let mut conf0 = Self::I2C_MST_ANA_CONF0_REG.read_volatile();
            conf0 &= !Self::I2C_MST_BBPLL_STOP_FORCE_LOW;
            conf0 |= Self::I2C_MST_BBPLL_STOP_FORCE_HIGH;
            Self::I2C_MST_ANA_CONF0_REG.write_volatile(conf0);
        }
    }

    fn bbpll_set_analog_480m() {
        const DIV_REF: u32 = 0;
        const DIV7_0: u32 = 8;
        const DR1: u32 = 0;
        const DR3: u32 = 0;
        const DCHGP: u32 = 5;
        const DCUR: u32 = 3;
        const VCO_DBIAS: u32 = 3;
        const OC_DCHGP_LSB: u32 = 4;
        const OC_DLREF_SEL_LSB: u32 = 6;
        const OC_DHREF_SEL_LSB: u32 = 4;

        unsafe {
            rom_i2c_writeReg(
                Self::I2C_BBPLL,
                Self::I2C_BBPLL_HOSTID,
                Self::I2C_BBPLL_REG4,
                Self::I2C_BBPLL_MODE_HF_480M,
            );
            rom_i2c_writeReg(
                Self::I2C_BBPLL,
                Self::I2C_BBPLL_HOSTID,
                Self::I2C_BBPLL_OC_REF,
                (DCHGP << OC_DCHGP_LSB) | DIV_REF,
            );
            rom_i2c_writeReg(
                Self::I2C_BBPLL,
                Self::I2C_BBPLL_HOSTID,
                Self::I2C_BBPLL_OC_DIV_REG,
                DIV7_0,
            );
            // Masked writes keep I2C_BBPLL_EN_USB in OC_DR.
            rom_i2c_writeReg_Mask(
                Self::I2C_BBPLL,
                Self::I2C_BBPLL_HOSTID,
                Self::I2C_BBPLL_OC_DR,
                2,
                0,
                DR1,
            );
            rom_i2c_writeReg_Mask(
                Self::I2C_BBPLL,
                Self::I2C_BBPLL_HOSTID,
                Self::I2C_BBPLL_OC_DR,
                6,
                4,
                DR3,
            );
            rom_i2c_writeReg(
                Self::I2C_BBPLL,
                Self::I2C_BBPLL_HOSTID,
                Self::I2C_BBPLL_REG6,
                (1 << OC_DLREF_SEL_LSB) | (3 << OC_DHREF_SEL_LSB) | DCUR,
            );
            rom_i2c_writeReg_Mask(
                Self::I2C_BBPLL,
                Self::I2C_BBPLL_HOSTID,
                Self::I2C_BBPLL_REG9,
                1,
                0,
                VCO_DBIAS,
            );
        }
    }

    /// Power the 480 MHz BBPLL. Mirrors esp-hal `enable_pll_clk_impl`.
    ///
    /// USB-Serial-JTAG already holds a ROM lock (`CAL_DONE`). Analog rewrite
    /// plus `calibration_stop` then glitches that PLL. Only run analog
    /// calibration when the lock is not already present.
    fn enable_pll_clk() {
        unsafe {
            Self::ANA_CONFIG_REG
                .write_volatile(Self::ANA_CONFIG_REG.read_volatile() & !Self::I2C_BBPLL_M);
            Self::RTC_CNTL_OPTIONS0_REG.write_volatile(
                Self::RTC_CNTL_OPTIONS0_REG.read_volatile() & !Self::RTC_CNTL_BBPLL_PD_M,
            );

            Self::SYSTEM_CPU_PER_CONF_REG.write_volatile(
                Self::SYSTEM_CPU_PER_CONF_REG.read_volatile() | Self::SYSTEM_PLL_FREQ_SEL_M,
            );
        }

        if Self::bbpll_cal_done() {
            return;
        }

        Self::bbpll_set_analog_480m();
        Self::bbpll_calibration_start();
        while !Self::bbpll_cal_done() {}
        unsafe { ets_delay_us(10) };
        Self::bbpll_calibration_stop();
    }

    pub fn restore(&self) {
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
