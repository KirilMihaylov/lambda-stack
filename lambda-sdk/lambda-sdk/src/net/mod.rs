use std::num::NonZeroU64;

use anyhow::bail;

use crate::interops::read_with_id;
use crate::interops::{Pointer, SlicePointer, StringPointer};

mod external;

#[repr(packed, C)]
pub struct Header<'r> {
    pub name: StringPointer<'r>,
    pub value: StringPointer<'r>,
}

#[repr(packed, C)]
pub struct Request<'r> {
    pub method: StringPointer<'r>,
    pub url: StringPointer<'r>,
    pub headers: SlicePointer<'r, Header<'r>>,
    pub body: SlicePointer<'r, u8>,
}

pub fn send_request(request: &Request<'_>) -> anyhow::Result<Response> {
    let id: u64 = unsafe { external::send_request(Pointer::from(request)) };

    if let Some(id) = NonZeroU64::new(id) {
        Ok(Response {
            id,
            length: unsafe { external::response_data_length(id) },
        })
    } else {
        bail!("There is a response to a previous request that has not been dropped!");
    }
}

pub struct Response {
    id: NonZeroU64,
    length: u64,
}

impl Response {
    pub fn status_code(&self) -> u16 {
        unsafe { external::response_status_code(self.id) }
            .try_into()
            .expect("Expected 16-bit integer, but got value that exceeds valid range!")
    }

    pub fn unread_length(&self) -> u64 {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    #[inline]
    pub fn read<'r>(&'_ mut self, buf: &'r mut [u8]) -> &'r [u8] {
        read_with_id(
            external::response_data,
            external::response_data_length,
            self.id,
            &mut self.length,
            buf,
        )
    }

    pub fn skip_over(&mut self, length: u64) {
        let min_length: u64 = self.length.min(length);

        self.length -= min_length;

        unsafe { external::drop_some_response_data(self.id, min_length) };
    }
}

impl Drop for Response {
    fn drop(&mut self) {
        unsafe { external::drop_response(self.id) }
    }
}
