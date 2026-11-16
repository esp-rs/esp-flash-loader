use crate::{
    efuse::{read_field, EfuseInfo},
    flash::MemSpi,
    rom::RomDataTables,
};

// Max of 32MB
pub const MAX_FLASH_SIZE: u32 = 0x2000000;

// TODO: fill from H4 ROM ELF when available.
pub const ROM_DATA_TABLES: RomDataTables = &[];

pub const ROM_TABLE_ENTRY_SIZE: u32 = 12;

pub const EFUSE_INFO: EfuseInfo = EfuseInfo {
    // EFUSE peripheral base = 0x600B1800; readable data starts at offset 0x2C
    block0: 0x600B_1800 + 0x2C,
    block_sizes: &[6, 6, 8, 8, 8, 8, 8, 8, 8, 8, 8],
};

pub const MEM_SPI: MemSpi = MemSpi {
    // SPIMEM1 (SPI1 flash controller) base address
    base: 0x6009_9000,
    cmd: 0x00,
    addr: 0x04,
    ctrl: 0x08,
    user: 0x18,
    user1: 0x1C,
    user2: 0x20,
    miso_dlen: 0x28,
    data_buf_0: 0x58,
};

unsafe extern "C" {
    fn ets_get_cpu_frequency() -> u32;
    fn ets_update_cpu_frequency(ticks_per_us: u32);
}

pub struct CpuSaveState {
    saved: [u32; 3],
    saved_ticks_per_us: u32,
    modified: bool,
}

impl CpuSaveState {
    const PCR: u32 = 0x6009_4000;
    const SYSCLK_CONF: *mut u32 = (Self::PCR + 0x114) as *mut u32;
    const CPU_FREQ_CONF: *mut u32 = (Self::PCR + 0x11C) as *mut u32;
    const AHB_FREQ_CONF: *mut u32 = (Self::PCR + 0x120) as *mut u32;
    const PLL_DIV_CLK_EN: *const u32 = (Self::PCR + 0x12C) as *const u32;
    const BUS_CLK_UPDATE: *mut u32 = (Self::PCR + 0x148) as *mut u32;

    const SOC_CLK_SEL_MASK: u32 = 3 << 16;
    const SOC_CLK_SEL_PLL: u32 = 3 << 16;
    const DIV_NUM_MASK: u32 = 0xFF;
    const PLL_96M_CLK_EN: u32 = 1 << 0;
    const BUS_CLOCK_UPDATE: u32 = 1 << 0;

    // AHB_CLK is divided from CPU_CLK and must stay at or below 32 MHz. Each field holds the
    // divider minus one.
    const CPU_MHZ: u32 = 96;
    const CPU_DIV: u32 = 0;
    const AHB_DIV: u32 = 2;

    pub const fn new() -> Self {
        CpuSaveState {
            saved: [0; 3],
            saved_ticks_per_us: 0,
            modified: false,
        }
    }

    fn regs() -> [*mut u32; 3] {
        [Self::CPU_FREQ_CONF, Self::AHB_FREQ_CONF, Self::SYSCLK_CONF]
    }

    fn update_bus_clocks() {
        unsafe {
            Self::BUS_CLK_UPDATE.write_volatile(Self::BUS_CLOCK_UPDATE);
            while Self::BUS_CLK_UPDATE.read_volatile() & Self::BUS_CLOCK_UPDATE != 0 {}
        }
    }

    pub fn set_max_cpu_clock(&mut self) {
        // The BBPLL also clocks the USB-JTAG PHY, so it runs while the probe is attached. Its
        // calibration status reads as 0 here, so only the 96 MHz output enable is checked.
        if unsafe { Self::PLL_DIV_CLK_EN.read_volatile() } & Self::PLL_96M_CLK_EN == 0 {
            return;
        }

        for (saved, reg) in self.saved.iter_mut().zip(Self::regs()) {
            *saved = unsafe { reg.read_volatile() };
        }
        self.saved_ticks_per_us = unsafe { ets_get_cpu_frequency() };
        self.modified = true;

        let set_field = |reg: *mut u32, mask: u32, value: u32| unsafe {
            reg.write_volatile((reg.read_volatile() & !mask) | value);
        };
        set_field(Self::CPU_FREQ_CONF, Self::DIV_NUM_MASK, Self::CPU_DIV);
        set_field(Self::AHB_FREQ_CONF, Self::DIV_NUM_MASK, Self::AHB_DIV);
        set_field(
            Self::SYSCLK_CONF,
            Self::SOC_CLK_SEL_MASK,
            Self::SOC_CLK_SEL_PLL,
        );
        Self::update_bus_clocks();
        unsafe { ets_update_cpu_frequency(Self::CPU_MHZ) };
    }

    pub fn configure_flash(&mut self) {}

    pub fn restore(&self) {
        if !self.modified {
            return;
        }
        for (saved, reg) in self.saved.iter().zip(Self::regs()) {
            unsafe { reg.write_volatile(*saved) };
        }
        Self::update_bus_clocks();
        unsafe { ets_update_cpu_frequency(self.saved_ticks_per_us) };
    }
}

// EFUSE_BLK1, bit 118, 2 bits (WAFER_VERSION_MAJOR)
pub fn major_chip_version() -> u8 {
    read_field::<1, 118, 2>()
}

// EFUSE_BLK1, bit 114, 4 bits (WAFER_VERSION_MINOR)
pub fn minor_chip_version() -> u8 {
    read_field::<1, 114, 4>()
}

/// Ensures that data (e.g. constants) are accessed through the data bus.
pub unsafe fn read_via_data_bus(s: &u8) -> u8 {
    unsafe { core::ptr::read(s as *const u8) }
}
