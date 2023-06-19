use std::num::NonZeroU64;

use crate::interops::Pointer;

#[link(wasm_import_module = "sdk::vault")]
extern "C" {
    #[cfg_attr(target_pointer_width = "32", link_name = "fetch_secret~32")]
    #[cfg_attr(target_pointer_width = "64", link_name = "fetch_secret~64")]
    pub(super) fn fetch_secret(identifier: Pointer<'_, u8, false>, identifier_len: usize) -> u64;

    #[link_name = "secret_length"]
    pub(super) fn secret_length(id: NonZeroU64) -> u64;

    #[cfg_attr(target_pointer_width = "32", link_name = "read_secret~32")]
    #[cfg_attr(target_pointer_width = "64", link_name = "read_secret~64")]
    pub(super) fn read_secret(id: NonZeroU64, buf: Pointer<'_, u8, true>, buf_len: usize) -> usize;

    #[link_name = "drop_secret"]
    pub(super) fn drop_secret(id: NonZeroU64);
}
