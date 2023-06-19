use std::num::NonZeroU64;

use crate::interops::Pointer;

#[link(wasm_import_module = "sdk::context")]
extern "C" {
    #[link_name = "sender_username_length"]
    pub(super) fn sender_username_length() -> Option<NonZeroU64>;

    #[cfg_attr(target_pointer_width = "32", link_name = "sender_username~32")]
    #[cfg_attr(target_pointer_width = "64", link_name = "sender_username~64")]
    pub(super) fn sender_username(buf: Pointer<'_, u8, true>, buf_len: usize, offset: u64)
        -> usize;
}
