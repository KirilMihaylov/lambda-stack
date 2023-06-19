use std::num::NonZeroU64;

use crate::interops::Pointer;

#[link(wasm_import_module = "sdk::io")]
extern "C" {
    #[link_name = "receive_request_data_id"]
    pub(super) fn receive_request_data_id() -> u64;

    #[link_name = "request_data_length"]
    pub(super) fn request_data_length(id: NonZeroU64) -> u64;

    #[cfg_attr(target_pointer_width = "32", link_name = "read_request_data~32")]
    #[cfg_attr(target_pointer_width = "64", link_name = "read_request_data~64")]
    pub(super) fn read_request_data(
        id: NonZeroU64,
        buf: Pointer<'_, u8, true>,
        buf_len: usize,
    ) -> usize;

    #[link_name = "set_response_is_error"]
    pub(super) fn set_response_is_error();

    #[cfg_attr(target_pointer_width = "32", link_name = "write_response_data~32")]
    #[cfg_attr(target_pointer_width = "64", link_name = "write_response_data~64")]
    pub(super) fn write_response_data(buf: Pointer<'_, u8, false>, buf_len: usize);
}
