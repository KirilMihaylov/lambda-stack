#[macro_export]
macro_rules! entry {
    ($main: expr) => {
        #[no_mangle]
        pub extern "C" fn entry() {
            $crate::panic::install_handler();

            let _: () = $main();
        }
    };
}
