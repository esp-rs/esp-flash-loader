pub struct EfuseInfo {
    pub block0: u32,
    /// In words
    pub block_sizes: &'static [u32],
}

pub fn read_field<const BLOCK: usize, const BIT_START: u32, const BIT_COUNT: u32>() -> u8 {
    let info = crate::chip::EFUSE_INFO;

    let word_offset = BIT_START / 32;
    let bit_offset = BIT_START % 32;

    let mask = (1 << BIT_COUNT) - 1;
    let bit_mask = mask << bit_offset;

    let block_offset = info.block_sizes.iter().take(BLOCK).sum::<u32>();
    let address = info.block0 + block_offset * 4 + word_offset * 4;
    let value =
        (unsafe { core::ptr::read_volatile(address as *const u32) } & bit_mask) >> bit_offset;

    value as u8
}

pub fn read_chip_revision() -> u32 {
    crate::chip::major_chip_version() as u32 * 100 + crate::chip::minor_chip_version() as u32
}
