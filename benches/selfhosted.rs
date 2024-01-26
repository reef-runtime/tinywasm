mod util;
use criterion::{criterion_group, criterion_main, Criterion};
use tinywasm::types::TinyWasmModule;
use util::tinywasm_module;

fn run_tinywasm(module: TinyWasmModule) {
    use tinywasm::*;
    let module = Module::from(module);
    let mut store = Store::default();
    let mut imports = Imports::default();
    imports.define("env", "printi32", Extern::typed_func(|_: FuncContext<'_>, _: i32| Ok(()))).expect("define");
    let instance = module.instantiate(&mut store, Some(imports)).expect("instantiate");
    let hello = instance.exported_func::<(), ()>(&mut store, "hello").expect("exported_func");
    hello.call(&mut store, ()).expect("call");
}

fn run_wasmi() {
    use wasmi::*;
    let engine = Engine::default();
    let module = wasmi::Module::new(&engine, TINYWASM).expect("wasmi::Module::new");
    let mut store = Store::new(&engine, ());
    let mut linker = <Linker<()>>::new(&engine);
    linker.define("env", "printi32", Func::wrap(&mut store, |_: Caller<'_, ()>, _: i32| {})).expect("define");
    let instance = linker.instantiate(&mut store, &module).expect("instantiate").start(&mut store).expect("start");
    let hello = instance.get_typed_func::<(), ()>(&mut store, "hello").expect("get_typed_func");
    hello.call(&mut store, ()).expect("call");
}

const TINYWASM: &[u8] = include_bytes!("../examples/rust/out/tinywasm.wasm");
fn criterion_benchmark(c: &mut Criterion) {
    let module = tinywasm_module(TINYWASM);

    c.bench_function("tinywasm", |b| b.iter(|| run_tinywasm(module.clone())));
    c.bench_function("wasmi", |b| b.iter(|| run_wasmi()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
