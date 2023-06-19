use crate::interops::Pointer;

mod external;

pub fn debug_str(s: &str) {
    unsafe { external::debug_str(Pointer::from(s.as_bytes()).into(), s.len()) }
}
