use std::num::NonZeroU64;

use crate::interops::Pointer;

use super::Request;

#[link(wasm_import_module = "sdk::net")]
extern "C" {
    #[cfg_attr(target_pointer_width = "32", link_name = "send_request~32")]
    #[cfg_attr(target_pointer_width = "64", link_name = "send_request~64")]
    pub(super) fn send_request(request: Pointer<'_, Request<'_>, false>) -> u64;

    #[link_name = "response_status_code"]
    pub(super) fn response_status_code(id: NonZeroU64) -> u32;

    #[link_name = "response_data_length"]
    pub(super) fn response_data_length(id: NonZeroU64) -> u64;

    #[cfg_attr(target_pointer_width = "32", link_name = "response_data~32")]
    #[cfg_attr(target_pointer_width = "64", link_name = "response_data~64")]
    pub(super) fn response_data(
        id: NonZeroU64,
        buf: Pointer<'_, u8, true>,
        buf_len: usize,
    ) -> usize;

    #[link_name = "drop_some_response_data"]
    pub(super) fn drop_some_response_data(id: NonZeroU64, length: u64);

    #[link_name = "drop_response"]
    pub(super) fn drop_response(id: NonZeroU64);
}
