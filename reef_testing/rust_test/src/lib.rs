mod reef {
    #[link(wasm_import_module = "reef")]
    extern "C" {
        fn log(pointer: *const u8, length: i32);
    }

    pub fn reef_log(msg: &str) {
        unsafe { log(msg.as_ptr(), msg.len() as i32) }
    }
}

#[no_mangle]
pub extern "C" fn reef_main() -> i32 {
    let msg = "Hello World!";

    reef::reef_log(msg);

    return 42;
}
