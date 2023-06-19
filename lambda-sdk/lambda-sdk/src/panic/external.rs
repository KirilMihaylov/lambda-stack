use crate::interops::Pointer;

#[link(wasm_import_module = "sdk::panic")]
extern "C" {
    #[cfg_attr(target_pointer_width = "32", link_name = "panic~32")]
    #[cfg_attr(target_pointer_width = "64", link_name = "panic~64")]
    pub(super) fn panic(buf: Pointer<'_, u8, false>, buf_len: usize) -> !;
}
