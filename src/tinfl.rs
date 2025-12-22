use core::mem::MaybeUninit;

use crate::ERROR_BASE_INTERNAL;

extern "C" {
    /// Main low-level decompressor coroutine function. This is the only function actually needed
    /// for decompression. All the other functions are just high-level helpers for improved
    /// usability.
    /// This is a universal API, i.e. it can be used as a building block to build any desired higher
    /// level decompression API. In the limit case, it can be called once per every byte input or
    /// output.
    fn tinfl_decompress(
        decompressor: *mut TinflDecompressor,
        next_in: *const u8,
        in_bytes: *mut usize,
        out_buf_start: *mut u8,
        next_out: *mut u8,
        out_bytes: *mut usize,
        flags: u32,
    ) -> TinflStatus;
}

type TinflStatus = i8;
// const TINFL_STATUS_FAILED_CANNOT_MAKE_PROGRESS: TinflStatus = -4;
// const TINFL_STATUS_BAD_PARAM: TinflStatus = -3;
// const TINFL_STATUS_ADLER32_MISMATCH: TinflStatus = -2;
// const TINFL_STATUS_FAILED: TinflStatus = -1;
pub const TINFL_STATUS_DONE: TinflStatus = 0;
pub const TINFL_STATUS_NEEDS_MORE_INPUT: TinflStatus = 1;
// const TINFL_STATUS_HAS_MORE_OUTPUT: TinflStatus = 2;

const TINFL_MAX_HUFF_SYMBOLS_0: usize = 288;
const TINFL_MAX_HUFF_TABLES: usize = 3;
const TINFL_MAX_HUFF_SYMBOLS_1: usize = 32;
const TINFL_FAST_LOOKUP_BITS: usize = 10;
const TINFL_FAST_LOOKUP_SIZE: usize = 1 << TINFL_FAST_LOOKUP_BITS;

// Decompression flags used by tinfl_decompress().

/// If set, the input has a valid zlib header and ends with an adler32 checksum (it's a valid zlib
/// stream). Otherwise, the input is a raw deflate stream.
const TINFL_FLAG_PARSE_ZLIB_HEADER: u32 = 1;

/// If set, there are more input bytes available beyond the end of the supplied input buffer.
/// If clear, the input buffer contains all remaining input.
const TINFL_FLAG_HAS_MORE_INPUT: u32 = 2;

/// If set, the output buffer is large enough to hold the entire decompressed stream.
/// If clear, the output buffer is at least the size of the dictionary (typically 32KB).
// const TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF: u32 = 4;

/// Force adler-32 checksum computation of the decompressed bytes.
// const TINFL_FLAG_COMPUTE_ADLER32: u32 = 8;

#[repr(C)]
struct TinflHuffTable {
    m_code_size: [u8; TINFL_MAX_HUFF_SYMBOLS_0],
    m_look_up: [u16; TINFL_FAST_LOOKUP_SIZE],
    m_tree: [u16; TINFL_MAX_HUFF_SYMBOLS_0 * 2],
}

#[repr(C)]
pub struct TinflDecompressor {
    m_state: u32,
    m_num_bits: MaybeUninit<u32>,
    m_zhdr0: MaybeUninit<u32>,
    m_zhdr1: MaybeUninit<u32>,
    m_z_adler32: MaybeUninit<u32>,
    m_final: MaybeUninit<u32>,
    m_type: MaybeUninit<u32>,
    m_check_adler32: MaybeUninit<u32>,
    m_dist: MaybeUninit<u32>,
    m_counter: MaybeUninit<u32>,
    m_num_extra: MaybeUninit<u32>,
    m_table_sizes: MaybeUninit<[u32; TINFL_MAX_HUFF_TABLES]>,
    m_bit_buf: MaybeUninit<u32>,
    m_dist_from_out_buf_start: MaybeUninit<usize>,
    m_tables: MaybeUninit<[TinflHuffTable; TINFL_MAX_HUFF_TABLES]>,
    m_raw_header: MaybeUninit<[u8; 4]>,
    m_len_codes: MaybeUninit<[u8; TINFL_MAX_HUFF_SYMBOLS_0 + TINFL_MAX_HUFF_SYMBOLS_1 + 137]>,
}

impl TinflDecompressor {
    pub const fn new() -> Self {
        unsafe {
            let mut this = MaybeUninit::<Self>::uninit();
            (*this.as_mut_ptr()).m_state = 0;
            this.assume_init()
        }
    }
}

pub struct OutBuffer {
    buffer: MaybeUninit<[u8; 32768]>,
    len: usize,
}

impl OutBuffer {
    pub const fn new() -> Self {
        Self {
            buffer: MaybeUninit::uninit(),
            len: 0,
        }
    }

    fn space(&self) -> usize {
        unsafe { self.buffer.assume_init() }.len() - self.len
    }

    unsafe fn pointers(&mut self) -> (*mut u8, *mut u8) {
        let write_idx = self.len;
        let start = core::ptr::addr_of_mut!(self.buffer).cast();

        (start, start.add(write_idx))
    }

    pub fn full(&self) -> bool {
        self.space() == 0
    }

    pub fn take<R>(&mut self, out: impl FnOnce(&[u8]) -> R) -> R {
        let data = unsafe {
            // self.len is always <= self.buffer.len()
            let len = core::mem::take(&mut self.len);
            self.buffer.assume_init_mut().get_unchecked(..len)
        };

        out(data)
    }
}

impl TinflDecompressor {
    pub fn decompress(&mut self, input: &mut &[u8], out: &mut OutBuffer, last: bool) -> i32 {
        let flags = if last {
            TINFL_FLAG_PARSE_ZLIB_HEADER
        } else {
            TINFL_FLAG_PARSE_ZLIB_HEADER | TINFL_FLAG_HAS_MORE_INPUT
        };

        let mut in_bytes = input.len();
        let data_buf = input.as_ptr();

        let mut out_bytes = out.space();
        let (out_buf, next_out) = unsafe { out.pointers() };

        let status = unsafe {
            tinfl_decompress(
                self,
                data_buf,
                &mut in_bytes,
                out_buf,
                next_out,
                &mut out_bytes,
                flags,
            )
        } as i32;

        if in_bytes > input.len() {
            // tinfl_decompress() shouldn't have consumed more bytes than we gave it
            // but let's not trust it
            return ERROR_BASE_INTERNAL - 2;
        }

        // Consume processed input
        *input = &input[in_bytes..];

        // Update output buffer
        out.len += out_bytes;

        status
    }
}
