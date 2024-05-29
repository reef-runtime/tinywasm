use std::io::ErrorKind;
use std::str::FromStr;

use argh::FromArgs;
// use args::WasmArg;
use color_eyre::eyre::Result;
use tinywasm::{CallResultTyped, Instance};
use tinywasm::{Extern, FuncContext, MemoryStringExt};
use tinywasm::{Imports, Module};

#[derive(FromArgs)]
/// TinyWasm CLI
struct TinyWasmCli {
    #[argh(subcommand)]
    nested: TinyWasmSubcommand,

    /// log level
    #[argh(option, short = 'l', default = "\"info\".to_string()")]
    log_level: String,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum TinyWasmSubcommand {
    Run(Run),
}

enum Engine {
    Main,
}

impl FromStr for Engine {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "main" => Ok(Self::Main),
            _ => Err(format!("unknown engine: {}", s)),
        }
    }
}

#[derive(FromArgs)]
/// run a wasm file
#[argh(subcommand, name = "run")]
struct Run {
    /// wasm file to run
    #[argh(positional)]
    wasm_file: String,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let args: TinyWasmCli = argh::from_env();

    let cwd = std::env::current_dir()?;

    match args.nested {
        TinyWasmSubcommand::Run(Run { wasm_file }) => {
            let path = cwd.join(wasm_file.clone());
            let module = match wasm_file.ends_with(".wat") {
                true => return Err(color_eyre::eyre::eyre!("wat support is not enabled in this build")),
                false => tinywasm::parse_bytes(&std::fs::read(path)?)?,
            };

            run(module)
        }
    }
}

fn run(module: Module) -> Result<()> {
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
        Extern::typed_func(|mut _ctx: FuncContext<'_>, percent: i32| {
            if !(0..=100).contains(&percent) {
                return Err(tinywasm::Error::Io(std::io::Error::new(
                    ErrorKind::AddrNotAvailable,
                    "Invalid range: percentage must be in 0..=100",
                )));
            }

            println!("REEF_REPORT_PROGRESS: {percent}");
            Ok(())
        }),
    )?;

    let max_cycles = 10;

    let entry_fn_name = "reef_main";

    let instance = Instance::instantiate(module, imports)?;

    let main_fn = instance.exported_func::<i32, i32>(entry_fn_name).unwrap();
    let mut exec_handle = main_fn.call(0)?;

    let mut cycles = 0;

    loop {
        cycles += 1;

        let run_res = exec_handle.run(max_cycles).unwrap();

        match run_res {
            CallResultTyped::Done(res) => {
                println!("finished: {res:?}");
                break;
            }
            CallResultTyped::Incomplete => {}
        }
    }

    println!("Took {cycles} rounds");

    Ok(())
}
