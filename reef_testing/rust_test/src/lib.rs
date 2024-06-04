mod reef {
    #[link(wasm_import_module = "reef")]
    extern "C" {
        fn log(pointer: *const u8, length: i32);
    }

    pub fn reef_log(msg: &str) {
        unsafe { log(msg.as_ptr(), msg.len() as i32) }
    }

    #[link(wasm_import_module = "reef")]
    extern "C" {
        fn progress(percent: f32);
    }

    pub fn reef_progress(done: f32) {
        unsafe { progress(done) }
    }
}

#[no_mangle]
pub extern "C" fn reef_main(arg: i32) -> i32 {
    std::panic::set_hook(Box::new(|info| {
        reef::reef_log(&format!("PANIC: {}", info.to_string()));
    }));

    let test_data = vec![arg as u8, 2, 3];
    run(&test_data);

    return 42;
}

fn run(dataset: &[u8]) {
    let msg = "Hello World!";

    reef::reef_log(msg);

    let struct1 = Struct1 { field1: 69 };
    let struct2 = Struct2 { field1: 69, field2: true };

    let msg1 = format!("struct1: {:?}", struct1);
    let msg2 = format!("struct2: {:?}", struct2);
    reef::reef_log(&msg1);
    reef::reef_log(&msg2);

    let the_struct: Box<dyn TestTrait> = if dataset[0] == 0 { Box::new(struct1) } else { Box::new(struct2) };
    reef::reef_log(&format!("out1: {}", run_test_trait2(&*the_struct)));
    reef::reef_log(&format!("out2: {}", run_test_trait1(the_struct)));

    const NUM: usize = 100_000;
    let mut a = 0;
    let mut b = 1;
    for i in 0..NUM {
        if i % 2000 == 0 {
            reef::reef_log(&format!("{b}"));
            reef::reef_progress(i as f32 / NUM as f32);
        }
        let c = a + b;
        a = b;
        b = c;
    }
}

trait TestTrait {
    fn get_num(&self) -> i32;
}

#[derive(Debug)]
struct Struct1 {
    field1: i32,
}

impl TestTrait for Struct1 {
    fn get_num(&self) -> i32 {
        self.field1
    }
}

#[derive(Debug)]
struct Struct2 {
    field1: i32,
    field2: bool,
}

impl TestTrait for Struct2 {
    fn get_num(&self) -> i32 {
        if self.field2 {
            self.field1
        } else {
            420
        }
    }
}

fn run_test_trait1(value: Box<dyn TestTrait>) -> String {
    value.get_num().to_string()
}

fn run_test_trait2(value: &dyn TestTrait) -> String {
    value.get_num().to_string()
}
