use std::io;

use argh::FromArgs;
// use args::WasmArg;
use color_eyre::eyre::Result;
use rkyv::AlignedVec;

use reef_interpreter::{
    error::Error,
    exec::CallResultTyped,
    imports::{Extern, FuncContext, Imports},
    parse_bytes,
    reference::MemoryStringExt,
    Instance, PAGE_SIZE,
};

/// Test CLI args
#[derive(FromArgs)]
struct CliArgs {
    /// wasm file to run
    #[argh(positional)]
    wasm_file: String,

    #[argh(positional)]
    wasm_arg: i32,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let args: CliArgs = argh::from_env();

    let cwd = std::env::current_dir()?;

    let path = cwd.join(args.wasm_file.clone());
    let module_bytes = match args.wasm_file.ends_with(".wat") {
        true => return Err(color_eyre::eyre::eyre!("wat support is not enabled in this build")),
        false => std::fs::read(path)?,
    };

    run(&module_bytes, args.wasm_arg)
}

const MAX_CYCLES: usize = 5000;
const ENTRY_NAME: &str = "reef_main";

fn run(module_bytes: &[u8], arg: i32) -> Result<()> {
    let mut serialized_state: Option<AlignedVec> = None;
    let mut cycles = 0;

    loop {
        cycles += 1;

        let module = parse_bytes(module_bytes)?;

        let mut imports = Imports::new();

        imports.define(
            "reef",
            "log",
            Extern::typed_func(|ctx: FuncContext<'_>, args: (i32, i32)| {
                let mem = ctx.exported_memory("memory")?;
                let ptr = args.0 as usize;
                let len = args.1 as usize;
                let string = mem.load_string(ptr, len)?;
                println!("REEF_LOG: {}", string);
                Ok(())
            }),
        )?;

        imports.define(
            "reef",
            "progress",
            Extern::typed_func(|mut _ctx: FuncContext<'_>, done: f32| {
                if !(0.0..=1.0).contains(&done) {
                    return Err(Error::Io(io::Error::other("Invalid range: progress must be between 0.0 and 1.0")));
                }

                println!("REEF_REPORT_PROGRESS: {done}");
                Ok(())
            }),
        )?;

        // this clone will not be happening in the final loop
        let (instance, stack) = match serialized_state.take() {
            None => (Instance::instantiate(module, imports)?, None),
            Some(state) => {
                let (instance, stack) = Instance::instantiate_with_state(module, imports, &state)?;
                (instance, Some(stack))
            }
        };

        let main_fn = instance.exported_func::<i32, i32>(ENTRY_NAME).unwrap();
        let mut exec_handle = main_fn.call(arg, stack)?;

        let run_res = exec_handle.run(MAX_CYCLES)?;

        match run_res {
            CallResultTyped::Done(res) => {
                println!("finished: {res:?}");
                println!("Took {cycles} rounds");

                break Ok(());
            }
            CallResultTyped::Incomplete => {
                if serialized_state.is_none() {
                    serialized_state = Some(AlignedVec::with_capacity(PAGE_SIZE * 2));
                }
                serialized_state = Some(exec_handle.serialize(serialized_state.take().unwrap())?);
                // println!("serialized {} bytes", serialized_state.as_ref().unwrap().len());
            }
        }
    }
}
