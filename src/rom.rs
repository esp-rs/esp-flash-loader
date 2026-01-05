pub type RomDataTables = &'static [RomDataTable];

pub struct RomDataTable {
    pub min_revision: u32,
    pub data_start: u32,
    pub data_end: u32,
    pub bss_start: u32,
    pub bss_end: u32,
}

struct DataTableDataEntry {
    dst_start: u32, // RAM
    dst_end: u32,   // RAM
    src: u32,       // ROM
}

impl DataTableDataEntry {
    // On chips where the table entry is 4 words, if the 4th word is 0, data
    // is only unpacked by the Pro core. If the 4th word is 1, data is unpacked
    // by both cores. This distinction is unnecessary for the flash loader.
    // We just need to jump over the 4th word.
    const ENTRY_SIZE: u32 = crate::chip::ROM_TABLE_ENTRY_SIZE;

    fn read(from: u32) -> Self {
        let dst_start = unsafe { *(from as *const u32) }; // RAM
        let dst_end = unsafe { *((from + 4) as *const u32) }; // RAM
        let src = unsafe { *((from + 8) as *const u32) }; // ROM

        DataTableDataEntry {
            dst_start,
            dst_end,
            src,
        }
    }

    fn init(&self) {
        let mut addr = self.src;
        let mut dst = self.dst_start;
        while dst < self.dst_end {
            unsafe { *(dst as *mut u32) = *(addr as *const u32) };
            addr += 4;
            dst += 4;
        }
    }
}

struct BssTableDataEntry {
    start: u32, // RAM
    end: u32,   // RAM
}

impl BssTableDataEntry {
    // On chips where the table entry is 3 words, if the 3rd word is 0, data
    // is only zeroed by the Pro core. If the 3rd word is 1, data is zeroed
    // by both cores. This distinction is unnecessary for the flash loader.
    // We just need to jump over the 3rd word.
    const ENTRY_SIZE: u32 = crate::chip::ROM_TABLE_ENTRY_SIZE - 4;

    fn read(from: u32) -> Self {
        let start = unsafe { *(from as *const u32) }; // RAM
        let end = unsafe { *((from + 4) as *const u32) }; // RAM

        BssTableDataEntry { start, end }
    }

    fn init(&self) {
        let mut addr = self.start;
        while addr < self.end {
            unsafe { *(addr as *mut u32) = 0 };
            addr += 4;
        }
    }
}

impl RomDataTable {
    fn init(&self) {
        unpack(self.data_start, self.data_end);
        zero(self.bss_start, self.bss_end);
    }
}

fn unpack(first: u32, end: u32) {
    // Data is stored in three/four-word sections:
    // - Data section start address in RAM
    // - Data section end address in RAM
    // - Source address in ROM
    // - Whether to unpack code on the second core
    let mut current = first;
    while current < end {
        DataTableDataEntry::read(current).init();
        current += DataTableDataEntry::ENTRY_SIZE;
    }
}

fn zero(first: u32, end: u32) {
    // Data is stored in three/four-word sections:
    // - Section start address in RAM
    // - Section end address in RAM
    // - Whether to zero code on the second core
    let mut current = first;
    while current < end {
        BssTableDataEntry::read(current).init();
        current += BssTableDataEntry::ENTRY_SIZE;
    }
}

pub fn init_rom_data() {
    let rev = crate::efuse::read_chip_revision();
    dprintln!("Chip revision: {}", rev);
    if let Some(table) = crate::chip::ROM_DATA_TABLES
        .iter()
        .filter(|table| table.min_revision <= rev)
        .last()
    {
        table.init();
    }
}
