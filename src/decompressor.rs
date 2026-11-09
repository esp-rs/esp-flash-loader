//! Decompresses the image, then writes it.

use crate::{
    flash,
    tinfl::{
        OutBuffer, TinflDecompressor, TINFL_STATUS_DONE, TINFL_STATUS_HAS_MORE_OUTPUT,
        TINFL_STATUS_NEEDS_MORE_INPUT,
    },
    ERROR_BASE_FLASH, ERROR_BASE_INTERNAL, ERROR_BASE_TINFL,
};

pub struct Decompressor {
    decompressor: TinflDecompressor,
    output: OutBuffer,
    image_start: Option<u32>,
    offset: u32,
    remaining_compressed: usize,
}

impl Decompressor {
    pub const fn new() -> Self {
        Self {
            image_start: None,
            offset: 0,
            output: OutBuffer::new(),
            remaining_compressed: 0,
            decompressor: TinflDecompressor::new(),
        }
    }

    fn reinit(&mut self, address: u32, compressed: u32) {
        self.image_start = Some(address);
        self.offset = 0;

        self.remaining_compressed = compressed as usize;

        self.decompressor = TinflDecompressor::new();
        self.output.take(|_| {});
    }

    fn decompress(&mut self, input: &[u8]) -> i32 {
        if self.remaining_compressed == 0 {
            return ERROR_BASE_INTERNAL - 3;
        }

        // We may have to cut off some padding bytes.
        let chunk_len = self.remaining_compressed.min(input.len());
        self.remaining_compressed -= chunk_len;

        // Signal tinfl_decompress() that this is the last chunk
        let last = self.remaining_compressed == 0;

        // Iterate through all the input, and all the output that it produces. The decompressor
        // can hold pending output after it consumed the last input byte.
        let mut input = &input[..chunk_len];
        let mut status = TINFL_STATUS_NEEDS_MORE_INPUT as i32;
        while status == TINFL_STATUS_HAS_MORE_OUTPUT as i32
            || (!input.is_empty() && status > TINFL_STATUS_DONE as i32)
        {
            status = self
                .decompressor
                .decompress(&mut input, &mut self.output, last);

            if status == TINFL_STATUS_DONE as i32 || self.output.full() {
                // We're either finished or the decompressor can't continue
                // until we flush the buffer.
                let flush_status = self.flush();

                if flush_status < 0 {
                    return flush_status;
                }
            }
        }

        if status < TINFL_STATUS_DONE as i32 {
            ERROR_BASE_TINFL + status
        } else {
            0
        }
    }

    pub fn flush(&mut self) -> i32 {
        let mut offset = self.offset;
        let address = self.image_start.unwrap_or(0) + offset;

        // Take buffer contents, write to flash and update offset.
        let status = self.output.take(|data| {
            offset += data.len() as u32;

            write_to_flash(address, data)
        });

        self.offset = offset;

        status
    }

    pub fn program(&mut self, address: u32, mut data: &[u8]) -> i32 {
        if self.image_start != Some(address) {
            if data.len() < 4 {
                // We don't have enough bytes to read the length
                return ERROR_BASE_INTERNAL - 4;
            }

            // Image length is prepended to the first chunk, cut it off.
            let (length_bytes, remaining) = data.split_at(4);
            data = remaining;

            let compressed_length = u32::from_le_bytes([
                length_bytes[0],
                length_bytes[1],
                length_bytes[2],
                length_bytes[3],
            ]);

            self.reinit(address, compressed_length);
        }
        self.decompress(data)
    }
}

/// Writes `data` one flash page at a time, and skips the pages that hold only 0xFF.
///
/// Programming 0xFF does not change NOR flash, and an image can expand to megabytes of it.
fn write_to_flash(mut address: u32, mut data: &[u8]) -> i32 {
    const FLASH_PAGE_SIZE: u32 = 256;

    while !data.is_empty() {
        let to_boundary = FLASH_PAGE_SIZE - address % FLASH_PAGE_SIZE;
        let len = data.len().min(to_boundary as usize);
        let (page, rest) = unsafe {
            // SAFETY: `len` is at most `data.len()`
            data.split_at_unchecked(len)
        };

        if !page.iter().all(|&byte| byte == 0xFF) {
            // The ROM returns 1 on error and 2 on timeout.
            let status = flash::write_flash(address, page);
            if status != 0 {
                return ERROR_BASE_FLASH - status;
            }
        }

        address += len as u32;
        data = rest;
    }

    0
}
