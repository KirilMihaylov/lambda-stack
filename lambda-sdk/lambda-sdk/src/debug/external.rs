use crate::interops::Pointer;

#[link(wasm_import_module = "sdk::debug")]
extern "C" {
    #[cfg_attr(target_pointer_width = "32", link_name = "debug_str~32")]
    #[cfg_attr(target_pointer_width = "64", link_name = "debug_str~64")]
    pub(super) fn debug_str(s: Pointer<'_, u8, false>, s_len: usize);
}
