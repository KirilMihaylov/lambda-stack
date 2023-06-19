use std::num::NonZeroU64;

use crate::interops::{self, Pointer};

mod external;

pub struct RequestData {
    id: NonZeroU64,
    length: u64,
}

impl RequestData {
    pub fn new() -> Option<Self> {
        NonZeroU64::new(unsafe { external::receive_request_data_id() }).map(|id: NonZeroU64| Self {
            id,
            length: unsafe { external::request_data_length(id) },
        })
    }

    #[inline]
    pub const fn len(&self) -> u64 {
        self.length
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    #[inline]
    pub fn read<'r>(&'_ mut self, buf: &'r mut [u8]) -> &'r [u8] {
        interops::read_with_id(
            external::read_request_data,
            external::request_data_length,
            self.id,
            &mut self.length,
            buf,
        )
    }
}

#[derive(Default)]
pub struct Response;

impl Response {
    #[inline]
    pub fn set_as_error() {
        unsafe { external::set_response_is_error() }
    }

    #[inline]
    pub fn write(buf: &[u8]) {
        unsafe { external::write_response_data(Pointer::from(buf).into(), buf.len()) }
    }
}
