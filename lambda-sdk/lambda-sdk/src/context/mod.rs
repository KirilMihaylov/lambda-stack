use std::num::NonZeroU64;

use crate::interops::Pointer;

mod external;

pub struct User {
    offset: u64,
    length: u64,
}

impl User {
    pub fn new() -> Option<Self> {
        unsafe { external::sender_username_length() }
            .map(NonZeroU64::get)
            .map(|length| Self { offset: 0, length })
    }

    pub const fn username_length(&self) -> u64 {
        self.length
    }

    #[inline]
    pub fn read<'r>(&mut self, buf: &'r mut [u8]) -> anyhow::Result<&'r [u8]> {
        let read_length: usize = if self.offset < self.length && buf.len() != 0 {
            let buf_len: usize = buf.len();

            let read_length: usize = unsafe {
                external::sender_username(
                    Pointer::<u8, true>::from(&mut buf[0]),
                    buf_len,
                    self.offset,
                )
            };

            self.length += u64::try_from(read_length)?;

            read_length
        } else {
            0
        };

        Ok(&buf[..read_length])
    }
}
