// esptool uses 16k for the buffer
pub const PAGE_SIZE: u32 = 0x4000;
pub const FLASH_BLOCK_SIZE: u32 = 65536;

#[allow(non_upper_case_globals)]
#[no_mangle]
#[used]
#[link_section = "DeviceData"]
pub static FlashDevice: FlashDeviceDescription = FlashDeviceDescription {
    vers: 0x0001,
    dev_name: [0u8; 128],
    dev_type: 5,
    dev_addr: 0x0,
    device_size: 0x1000000, // Max of 16MB
    page_size: PAGE_SIZE,
    _reserved: 0,
    empty: 0xFF,
    program_time_out: 1000,
    erase_time_out: 2000,
    flash_sectors: sectors(),
};

const fn sectors() -> [FlashSector; 512] {
    const SECTOR_END: FlashSector = FlashSector {
        size: 0xffff_ffff,
        address: 0xffff_ffff,
    };

    let mut sectors = [FlashSector::default(); 512];

    sectors[0] = FlashSector {
        size: FLASH_BLOCK_SIZE,
        address: 0x0,
    };
    sectors[1] = SECTOR_END;

    sectors
}

#[repr(C)]
pub struct FlashDeviceDescription {
    vers: u16,
    dev_name: [u8; 128],
    dev_type: u16,
    dev_addr: u32,
    device_size: u32,
    page_size: u32,
    _reserved: u32,
    empty: u8,
    program_time_out: u32,
    erase_time_out: u32,
    flash_sectors: [FlashSector; 512],
}

#[repr(C)]
#[derive(Copy, Clone)]
struct FlashSector {
    size: u32,
    address: u32,
}

impl FlashSector {
    const fn default() -> Self {
        FlashSector {
            size: 0,
            address: 0,
        }
    }
}
