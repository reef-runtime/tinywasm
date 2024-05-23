use crate::{runtime::RawWasmValue, unlikely, Function};
use alloc::{boxed::Box, format, string::String, string::ToString, vec, vec::Vec};
use tinywasm_types::{FuncType, ModuleInstanceAddr, ValType, WasmValue};

use crate::runtime::{CallFrame, Stack};
use crate::{Error, FuncContext, Result, Store};

#[derive(Debug)]
/// A function handle
pub struct FuncHandle {
    pub(crate) module_addr: ModuleInstanceAddr,
    pub(crate) addr: u32,
    pub(crate) ty: FuncType,

    /// The name of the function, if it has one
    pub name: Option<String>,
}

#[derive(Debug)]
pub enum CallResult {
    Done(Vec<WasmValue>),
    Incomplete(Stack),
}

impl FuncHandle {
    /// Call a function (Invocation)
    ///
    /// See <https://webassembly.github.io/spec/core/exec/modules.html#invocation>
    #[inline]
    pub fn call(
        &self,
        store: &mut Store,
        params: &[WasmValue],
        stack: Option<Stack>,
        max_cycles: usize,
    ) -> Result<CallResult> {
        // Comments are ordered by the steps in the spec
        // In this implementation, some steps are combined and ordered differently for performance reasons

        // 3. Let func_ty be the function type
        let func_ty = &self.ty;

        // 4. If the length of the provided argument values is different from the number of expected arguments, then fail
        if unlikely(func_ty.params.len() != params.len()) {
            return Err(Error::Other(format!(
                "param count mismatch: expected {}, got {}",
                func_ty.params.len(),
                params.len()
            )));
        }

        // 5. For each value type and the corresponding value, check if types match
        if !(func_ty.params.iter().zip(params).all(|(ty, param)| ty == &param.val_type())) {
            return Err(Error::Other("Type mismatch".into()));
        }

        let func_inst = store.get_func(self.addr)?;
        let wasm_func = match &func_inst.func {
            Function::Host(host_func) => {
                let func = &host_func.clone().func;
                let ctx = FuncContext { store, module_addr: self.module_addr };
                return Ok(CallResult::Done((func)(ctx, params)?));
            }
            Function::Wasm(wasm_func) => wasm_func,
        };

        let mut stack = match stack {
            None => {
                // 6. Let f be the dummy frame
                // 7. Push the frame f to the call stack
                // & 8. Push the values to the stack (Not needed since the call frame owns the values)
                let call_frame_params = params.iter().map(|v| RawWasmValue::from(*v));
                let call_frame = CallFrame::new(wasm_func.clone(), func_inst.owner, call_frame_params, 0);
                Stack::new(call_frame)
            }
            Some(old_stack) => old_stack,
        };

        // 9. Invoke the function instance
        let runtime = crate::runtime::interpreter::InterpreterRuntime {};
        if !runtime.exec(store, &mut stack, max_cycles)? {
            // panic!("{stack:?}");
            return Ok(CallResult::Incomplete(stack));
        }

        // Once the function returns:
        let result_m = func_ty.results.len();

        // 1. Assert: m values are on the top of the stack (Ensured by validation)
        assert!(stack.values.len() >= result_m);

        // 2. Pop m values from the stack
        let res = stack.values.last_n(result_m)?;

        // The values are returned as the results of the invocation.
        Ok(CallResult::Done(res.iter().zip(func_ty.results.iter()).map(|(v, ty)| v.attach_type(*ty)).collect()))
    }
}

#[derive(Debug)]
/// A typed function handle
pub struct FuncHandleTyped<P, R> {
    /// The underlying function handle
    pub func: FuncHandle,
    pub(crate) marker: core::marker::PhantomData<(P, R)>,
}

pub trait IntoWasmValueTuple {
    fn into_wasm_value_tuple(self) -> Vec<WasmValue>;
}

pub trait FromWasmValueTuple {
    fn from_wasm_value_tuple(values: &[WasmValue]) -> Result<Self>
    where
        Self: Sized;
}

#[derive(Debug)]
pub enum CallResultOuter<R: FromWasmValueTuple> {
    Done(R),
    Incomplete(Stack),
}

impl<P: IntoWasmValueTuple, R: FromWasmValueTuple> FuncHandleTyped<P, R> {
    /// Call a typed function
    pub fn call(
        &self,
        store: &mut Store,
        params: P,
        stack: Option<Stack>,
        max_cycles: usize,
    ) -> Result<CallResultOuter<R>> {
        // Convert params into Vec<WasmValue>
        let wasm_values = params.into_wasm_value_tuple();

        // Call the underlying WASM function
        let result = self.func.call(store, &wasm_values, stack, max_cycles)?;

        Ok(match result {
            CallResult::Done(values) => CallResultOuter::Done(R::from_wasm_value_tuple(&values)?),
            CallResult::Incomplete(stack) => CallResultOuter::Incomplete(stack),
        })
    }
}

macro_rules! impl_into_wasm_value_tuple {
    ($($T:ident),*) => {
        impl<$($T),*> IntoWasmValueTuple for ($($T,)*)
        where
            $($T: Into<WasmValue>),*
        {
            #[allow(non_snake_case)]
            #[inline]
            fn into_wasm_value_tuple(self) -> Vec<WasmValue> {
                let ($($T,)*) = self;
                vec![$($T.into(),)*]
            }
        }
    }
}

macro_rules! impl_into_wasm_value_tuple_single {
    ($T:ident) => {
        impl IntoWasmValueTuple for $T {
            #[inline]
            fn into_wasm_value_tuple(self) -> Vec<WasmValue> {
                vec![self.into()]
            }
        }
    };
}

macro_rules! impl_from_wasm_value_tuple {
    ($($T:ident),*) => {
        impl<$($T),*> FromWasmValueTuple for ($($T,)*)
        where
            $($T: TryFrom<WasmValue, Error = ()>),*
        {
            #[inline]
            fn from_wasm_value_tuple(values: &[WasmValue]) -> Result<Self> {
                #[allow(unused_variables, unused_mut)]
                let mut iter = values.iter();

                Ok((
                    $(
                        $T::try_from(
                            *iter.next()
                            .ok_or(Error::Other("Not enough values in WasmValue vector".to_string()))?
                        )
                        .map_err(|e| Error::Other(format!("FromWasmValueTuple: Could not convert WasmValue to expected type: {:?}", e,
                    )))?,
                    )*
                ))
            }
        }
    }
}

macro_rules! impl_from_wasm_value_tuple_single {
    ($T:ident) => {
        impl FromWasmValueTuple for $T {
            #[inline]
            fn from_wasm_value_tuple(values: &[WasmValue]) -> Result<Self> {
                #[allow(unused_variables, unused_mut)]
                let mut iter = values.iter();
                $T::try_from(*iter.next().ok_or(Error::Other("Not enough values in WasmValue vector".to_string()))?)
                    .map_err(|e| {
                        Error::Other(format!(
                            "FromWasmValueTupleSingle: Could not convert WasmValue to expected type: {:?}",
                            e
                        ))
                    })
            }
        }
    };
}

pub trait ValTypesFromTuple {
    fn val_types() -> Box<[ValType]>;
}

pub trait ToValType {
    fn to_val_type() -> ValType;
}

impl ToValType for i32 {
    fn to_val_type() -> ValType {
        ValType::I32
    }
}

impl ToValType for i64 {
    fn to_val_type() -> ValType {
        ValType::I64
    }
}

impl ToValType for f32 {
    fn to_val_type() -> ValType {
        ValType::F32
    }
}

impl ToValType for f64 {
    fn to_val_type() -> ValType {
        ValType::F64
    }
}

macro_rules! impl_val_types_from_tuple {
    ($($t:ident),+) => {
        impl<$($t),+> ValTypesFromTuple for ($($t,)+)
        where
            $($t: ToValType,)+
        {
            #[inline]
            fn val_types() -> Box<[ValType]> {
                Box::new([$($t::to_val_type(),)+])
            }
        }
    };
}

impl ValTypesFromTuple for () {
    #[inline]
    fn val_types() -> Box<[ValType]> {
        Box::new([])
    }
}

impl<T: ToValType> ValTypesFromTuple for T {
    #[inline]
    fn val_types() -> Box<[ValType]> {
        Box::new([T::to_val_type()])
    }
}

impl_from_wasm_value_tuple_single!(i32);
impl_from_wasm_value_tuple_single!(i64);
impl_from_wasm_value_tuple_single!(f32);
impl_from_wasm_value_tuple_single!(f64);

impl_into_wasm_value_tuple_single!(i32);
impl_into_wasm_value_tuple_single!(i64);
impl_into_wasm_value_tuple_single!(f32);
impl_into_wasm_value_tuple_single!(f64);

impl_val_types_from_tuple!(T1);
impl_val_types_from_tuple!(T1, T2);
impl_val_types_from_tuple!(T1, T2, T3);
impl_val_types_from_tuple!(T1, T2, T3, T4);
impl_val_types_from_tuple!(T1, T2, T3, T4, T5);
impl_val_types_from_tuple!(T1, T2, T3, T4, T5, T6);

impl_from_wasm_value_tuple!();
impl_from_wasm_value_tuple!(T1);
impl_from_wasm_value_tuple!(T1, T2);
impl_from_wasm_value_tuple!(T1, T2, T3);
impl_from_wasm_value_tuple!(T1, T2, T3, T4);
impl_from_wasm_value_tuple!(T1, T2, T3, T4, T5);
impl_from_wasm_value_tuple!(T1, T2, T3, T4, T5, T6);

impl_into_wasm_value_tuple!();
impl_into_wasm_value_tuple!(T1);
impl_into_wasm_value_tuple!(T1, T2);
impl_into_wasm_value_tuple!(T1, T2, T3);
impl_into_wasm_value_tuple!(T1, T2, T3, T4);
impl_into_wasm_value_tuple!(T1, T2, T3, T4, T5);
impl_into_wasm_value_tuple!(T1, T2, T3, T4, T5, T6);
