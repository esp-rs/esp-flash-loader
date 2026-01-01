use core::{cell::Cell, ptr};

use crate::chip::read_via_data_bus;

/// Specifies what to do when a channel doesn't have enough buffer space for a complete write.
#[derive(Eq, PartialEq)]
#[repr(usize)]
pub enum ChannelMode {
    /// Skip writing the data completely if it doesn't fit in its entirety.
    NoBlockSkip = 0,

    /// Write as much as possible of the data and ignore the rest.
    NoBlockTrim = 1,

    /// Block (spin) if the buffer is full. If within a critical section such as inside
    /// [`rprintln`], this will cause the application to freeze until the host reads from the
    /// buffer.
    BlockIfFull = 2,
}

// Note: this is zero-initialized in the initialization macro so all zeros must be a valid value
#[repr(C)]
pub struct RttHeader {
    id: [Cell<u8>; 16],
    max_up_channels: Cell<u32>,
    max_down_channels: Cell<u32>,
    // Followed in memory by:
    // up_channels: [Channel; max_up_channels]
    // down_channels: [Channel; down_up_channels]
}

impl RttHeader {
    /// Initializes the control block header.
    ///
    /// # Safety
    ///
    /// The arguments must correspond to the sizes of the arrays that follow the header in memory.
    pub unsafe fn init(&self, max_up_channels: usize, max_down_channels: usize) {
        self.max_up_channels.set(max_up_channels as u32);
        self.max_down_channels.set(max_down_channels as u32);

        // Copy the ID backward to avoid storing the magic string in the binary. The ID is
        // written backwards to make it less likely an unfinished control block is detected by the host.

        const MAGIC_STR_BACKWARDS: &[u8; 16] = b"\0\0\0\0\0\0TTR REGGES";

        for (idx, byte) in MAGIC_STR_BACKWARDS.iter().enumerate() {
            self.id[15 - idx].set(read_via_data_bus(byte));
        }
    }
}

// Note: this is zero-initialized in the initialization macro so all zeros must be a valid value
#[repr(C)]
pub struct Channel {
    name: Cell<*const u8>,
    buffer: Cell<*mut u8>,
    size: Cell<u32>,
    write: Cell<u32>,
    read: Cell<u32>,
    flags: Cell<u32>,
}

impl Channel {
    pub const fn new() -> Self {
        Self {
            name: Cell::new(ptr::null()),
            buffer: Cell::new(ptr::null_mut()),
            size: Cell::new(0),
            write: Cell::new(0),
            read: Cell::new(0),
            flags: Cell::new(0),
        }
    }

    /// Initializes the channel.
    ///
    /// # Safety
    ///
    /// The pointer arguments must point to a valid null-terminated name and writable buffer.
    pub unsafe fn init(&self, name: *const u8, mode: ChannelMode, buffer: *mut [u8]) {
        self.name.set(name);
        self.size.set(buffer.len() as u32);
        self.set_mode(mode);

        self.buffer.set(buffer.cast::<u8>());
    }

    /// Returns true on a non-null value of the (raw) buffer pointer
    pub fn is_initialized(&self) -> bool {
        !self.buffer.get().is_null()
    }

    pub fn mode(&self) -> ChannelMode {
        let mode = self.flags.get() & 3;

        match mode {
            0 => ChannelMode::NoBlockSkip,
            1 => ChannelMode::NoBlockTrim,
            2 => ChannelMode::BlockIfFull,
            _ => ChannelMode::NoBlockSkip,
        }
    }

    pub fn set_mode(&self, mode: ChannelMode) {
        self.flags.set(self.flags.get() & !3 | mode as u32);
    }

    pub(crate) fn read_pointers(&self) -> (usize, usize) {
        let write = self.write.get();
        let read = self.read.get();
        let size = self.size.get();

        if write >= size || read >= size {
            // Pointers have been corrupted. This doesn't happen in well-behaved programs, so
            // attempt to reset the buffer.

            self.write.set(0);
            self.read.set(0);
            return (0, 0);
        }

        (write as usize, read as usize)
    }

    fn writable_contiguous(&self, write: usize) -> usize {
        let read = self.read_pointers().1;

        if read > write {
            read - write - 1
        } else {
            let size = self.size.get() as usize;
            if read == 0 {
                size.saturating_sub(write).saturating_sub(1)
            } else {
                size.saturating_sub(write)
            }
        }
    }

    pub fn write(&self, mut buf: &[u8]) {
        let mut write = self.write.get() as usize;
        let size = self.size.get() as usize;
        loop {
            let count = self.writable_contiguous(write).min(buf.len());

            if count == 0 {
                // Buffer is full, or done
                let mode = self.mode();

                if buf.is_empty() || mode != ChannelMode::NoBlockSkip {
                    self.write.set(write as u32);
                }

                if buf.is_empty() || mode != ChannelMode::BlockIfFull {
                    return;
                }
                // Loop until there is space
                continue;
            }

            for (idx, byte) in buf.iter().enumerate().take(count) {
                unsafe {
                    ptr::write_volatile(self.buffer.get().add(write + idx), read_via_data_bus(byte))
                };
            }

            write += count;

            if write >= size {
                // Wrap around to start
                write = 0;
            }

            buf = &buf.get(count..).unwrap_or(&[]);
        }
    }
}

#[repr(C)]
pub struct RttControlBlock {
    header: RttHeader,
    up_channels: [Channel; 1],
    down_channels: [Channel; 0],
}

impl RttControlBlock {
    pub const fn new() -> Self {
        Self {
            header: RttHeader {
                id: [const { Cell::new(0) }; 16],
                max_up_channels: Cell::new(0),
                max_down_channels: Cell::new(0),
            },
            up_channels: [Channel::new(); 1],
            down_channels: [Channel::new(); 0],
        }
    }

    pub fn init(&self) {
        unsafe { self.header.init(1, 0) };
        unsafe {
            self.up_channels[0].init(
                b"Terminal\0".as_ptr(),
                ChannelMode::NoBlockSkip,
                &raw mut CHANNEL0_BUFFER,
            );
        }
    }
}

// Safety: the flash loader is single threaded.
unsafe impl Send for RttControlBlock {}
unsafe impl Sync for RttControlBlock {}

#[export_name = "_SEGGER_RTT"]
pub static CONTROL_BLOCK: RttControlBlock = RttControlBlock::new();
const BUFFER_SIZE: usize = 256;
pub static mut CHANNEL0_BUFFER: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];

pub struct RttLog;

impl ufmt::uWrite for RttLog {
    type Error = ();

    fn write_str(&mut self, s: &str) -> Result<(), ()> {
        if !CONTROL_BLOCK.up_channels[0].is_initialized() {
            CONTROL_BLOCK.init();
        }

        CONTROL_BLOCK.up_channels[0].write(s.as_bytes());

        Ok(())
    }
}
