//! Flash range signatures for the probe-rs flash compare.

use crate::{flash, md5::Md5Context, ERROR_BASE_INTERNAL};

/// The result of signing a range.
#[repr(u32)]
#[derive(Clone, Copy)]
pub enum SignatureStatus {
    /// The range contains data. The digest is valid.
    Signed = 0,
    /// All bytes of the range are 0xFF.
    Erased = 1,
}

/// One entry of the signature table. All values are little-endian.
#[repr(C)]
pub struct SignatureEntry {
    address: [u8; 4],
    size: [u8; 4],
    status: [u8; 4],
    digest: [u8; 16],
}

impl SignatureEntry {
    fn address(&self) -> u32 {
        u32::from_le_bytes(self.address)
    }

    fn size(&self) -> u32 {
        u32::from_le_bytes(self.size)
    }

    fn set_status(&mut self, status: SignatureStatus) {
        self.status = (status as u32).to_le_bytes();
    }

    /// Reads the range through `buffer`, and writes its status and MD5 digest.
    pub fn sign(&mut self, hasher: &mut Md5Context, buffer: &mut [u8]) -> Result<(), i32> {
        let start = self.address();
        let end = start + self.size();

        hasher.init();
        let mut erased = true;
        for chunk_start in (start..end).step_by(buffer.len()) {
            let len = (end - chunk_start).min(buffer.len() as u32);
            let Some(chunk) = buffer.get_mut(..len as usize) else {
                return Err(ERROR_BASE_INTERNAL - 3);
            };
            if flash::read_flash(chunk_start, chunk) != 0 {
                return Err(ERROR_BASE_INTERNAL - 2);
            }
            erased &= chunk.iter().all(|&byte| byte == 0xFF);
            hasher.update(chunk);
        }

        self.set_status(if erased {
            SignatureStatus::Erased
        } else {
            SignatureStatus::Signed
        });
        hasher.finalize(&mut self.digest);

        Ok(())
    }
}
