use tinywasm_types::WasmValue;

use crate::{runtime::Stack, CallResultTyped, FromWasmValueTuple, FuncHandle, Result};

#[derive(Debug)]
pub enum CallResult {
    Done(Vec<WasmValue>),
    Incomplete,
}

#[derive(Debug)]
pub struct ExecHandle {
    pub(crate) func_handle: FuncHandle,
    pub(crate) stack: Stack,
}

impl ExecHandle {
    pub fn run(&mut self, max_cycles: usize) -> Result<CallResult> {
        let runtime = crate::runtime::interpreter::Interpreter {};
        if !runtime.exec(&mut self.func_handle.instance, &mut self.stack, max_cycles)? {
            return Ok(CallResult::Incomplete);
        }

        // Once the function returns:
        let result_m = self.func_handle.ty.results.len();

        // 1. Assert: m values are on the top of the stack (Ensured by validation)
        assert!(self.stack.values.len() >= result_m);

        // 2. Pop m values from the stack
        let res = self.stack.values.last_n(result_m)?;

        // The values are returned as the results of the invocation.
        Ok(CallResult::Done(
            res.iter().zip(self.func_handle.ty.results.iter()).map(|(v, ty)| v.attach_type(*ty)).collect(),
        ))
    }
}

#[derive(Debug)]
pub struct ExecHandleTyped<R: FromWasmValueTuple> {
    pub(crate) exec_handle: ExecHandle,
    pub(crate) _marker: core::marker::PhantomData<R>,
}

impl<R: FromWasmValueTuple> ExecHandleTyped<R> {
    pub fn run(&mut self, max_cycles: usize) -> Result<CallResultTyped<R>> {
        // Call the underlying WASM function
        let result = self.exec_handle.run(max_cycles)?;

        Ok(match result {
            CallResult::Done(values) => CallResultTyped::Done(R::from_wasm_value_tuple(&values)?),
            CallResult::Incomplete => CallResultTyped::Incomplete,
        })
    }
}
