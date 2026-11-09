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
    } else if #[cfg(feature = "esp32p4")] {
        extern "C" {
            fn MD5Init(context: *mut Md5Context);
            fn MD5Update(context: *mut Md5Context, data: *const u8, len: u32);
            fn MD5Final(digest: *mut u8, context: *mut Md5Context);

            fn MD5Init_v3(context: *mut Md5Context);
            fn MD5Update_v3(context: *mut Md5Context, data: *const u8, len: u32);
            fn MD5Final_v3(digest: *mut u8, context: *mut Md5Context);
        }

        /// The v3.0 ROM has the MD5 functions at different addresses.
        fn is_v3() -> bool {
            crate::efuse::read_chip_revision() >= 300
        }

        impl Md5Context {
            pub fn init(&mut self) {
                if is_v3() {
                    unsafe { MD5Init_v3(self) };
                } else {
                    unsafe { MD5Init(self) };
                }
            }

            pub fn update(&mut self, data: &[u8]) {
                if is_v3() {
                    unsafe { MD5Update_v3(self, data.as_ptr(), data.len() as u32) };
                } else {
                    unsafe { MD5Update(self, data.as_ptr(), data.len() as u32) };
                }
            }

            pub fn finalize(&mut self, digest: &mut [u8; 16]) {
                if is_v3() {
                    unsafe { MD5Final_v3(digest.as_mut_ptr(), self) };
                } else {
                    unsafe { MD5Final(digest.as_mut_ptr(), self) };
                }
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
