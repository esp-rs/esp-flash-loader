//! MD5 from the ROM.

/// The ROM MD5 context.
///
/// The layout differs between ROM versions, but the size is 88 bytes in each.
#[repr(C, align(4))]
pub struct Md5Context([u8; 88]);

impl Md5Context {
    pub const fn new() -> Self {
        Self([0; 88])
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "esp32c2")] {
        extern "C" {
            fn mbedtls_md5_starts_ret(context: *mut Md5Context) -> i32;
            fn mbedtls_md5_update_ret(context: *mut Md5Context, data: *const u8, len: u32) -> i32;
            fn mbedtls_md5_finish_ret(context: *mut Md5Context, digest: *mut u8) -> i32;
        }

        impl Md5Context {
            pub fn init(&mut self) {
                unsafe { mbedtls_md5_starts_ret(self) };
            }

            pub fn update(&mut self, data: &[u8]) {
                unsafe { mbedtls_md5_update_ret(self, data.as_ptr(), data.len() as u32) };
            }

            pub fn finalize(&mut self, digest: &mut [u8; 16]) {
                unsafe { mbedtls_md5_finish_ret(self, digest.as_mut_ptr()) };
            }
        }
    } else {
        extern "C" {
            fn MD5Init(context: *mut Md5Context);
            fn MD5Update(context: *mut Md5Context, data: *const u8, len: u32);
            fn MD5Final(digest: *mut u8, context: *mut Md5Context);
        }

        impl Md5Context {
            pub fn init(&mut self) {
                unsafe { MD5Init(self) };
            }

            pub fn update(&mut self, data: &[u8]) {
                unsafe { MD5Update(self, data.as_ptr(), data.len() as u32) };
            }

            pub fn finalize(&mut self, digest: &mut [u8; 16]) {
                unsafe { MD5Final(digest.as_mut_ptr(), self) };
            }
        }
    }
}
