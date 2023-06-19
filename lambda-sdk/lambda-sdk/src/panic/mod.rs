use std::panic::PanicInfo;

use crate::interops::Pointer;

mod external;

pub fn install_handler() {
    std::panic::set_hook(Box::new(panic_handler))
}

fn panic_handler(info: &PanicInfo) {
    let info: String = info.to_string();

    unsafe { external::panic(Pointer::from(info.as_bytes()).into(), info.len()) }
}
