use core::mem::take;
use std::io;

use tinywasm_types::WasmValue;

use crate::{
    runtime::{RawWasmValue, Stack},
    CallResultTyped, FromWasmValueTuple, FuncHandle, Result,
};

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

    pub fn serialize(&mut self) -> Result<Vec<u8>> {
        let memory = &mut self.func_handle.instance.memories[0];
        let globals = self.func_handle.instance.globals.iter().map(|g| g.value).collect();
        let data = SerializationState { stack: take(&mut self.stack), memory: take(&mut memory.data), globals };

        let bytes: Vec<_> = rkyv::to_bytes::<_, 0x10000>(&data).map_err(io::Error::other)?.into();

        memory.data = data.memory;
        self.stack = data.stack;

        Ok(bytes)
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

    pub fn serialize(&mut self) -> Result<Vec<u8>> {
        self.exec_handle.serialize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[archive(check_bytes)]
pub(crate) struct SerializationState {
    pub(crate) stack: Stack,
    pub(crate) memory: Vec<u8>,
    pub(crate) globals: Vec<RawWasmValue>,
}
