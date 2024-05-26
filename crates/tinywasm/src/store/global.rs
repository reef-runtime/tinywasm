use alloc::{format, string::ToString};
use tinywasm_types::*;

use crate::{runtime::RawWasmValue, unlikely, Error, Result};

/// A WebAssembly Global Instance
///
/// See <https://webassembly.github.io/spec/core/exec/runtime.html#global-instances>
#[derive(Debug)]
pub(crate) struct GlobalInstance {
    pub(crate) value: RawWasmValue,
    pub(crate) ty: GlobalType,
}

impl GlobalInstance {
    pub(crate) fn new(ty: GlobalType, value: RawWasmValue) -> Self {
        Self { ty, value: value.into() }
    }

    #[inline]
    pub(crate) fn get(&self) -> WasmValue {
        self.value.attach_type(self.ty.ty)
    }

    pub(crate) fn set(&mut self, val: WasmValue) -> Result<()> {
        if unlikely(val.val_type() != self.ty.ty) {
            return Err(Error::Other(format!(
                "global type mismatch: expected {:?}, got {:?}",
                self.ty.ty,
                val.val_type()
            )));
        }

        if unlikely(!self.ty.mutable) {
            return Err(Error::Other("global is immutable".to_string()));
        }

        self.value = val.into();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_instance_get_set() {
        let global_type = GlobalType { ty: ValType::I32, mutable: true };
        let initial_value = RawWasmValue::from(10i32);

        let mut global_instance = GlobalInstance::new(global_type, initial_value);

        // Test `get`
        assert_eq!(global_instance.get(), WasmValue::I32(10), "global value should be 10");

        // Test `set` with correct type
        assert!(global_instance.set(WasmValue::I32(20)).is_ok(), "set should succeed");
        assert_eq!(global_instance.get(), WasmValue::I32(20), "global value should be 20");

        // Test `set` with incorrect type
        assert!(matches!(global_instance.set(WasmValue::F32(1.0)), Err(Error::Other(_))), "set should fail");

        // Test `set` on immutable global
        let immutable_global_type = GlobalType { ty: ValType::I32, mutable: false };
        let mut immutable_global_instance = GlobalInstance::new(immutable_global_type, initial_value);
        assert!(matches!(immutable_global_instance.set(WasmValue::I32(30)), Err(Error::Other(_))));
    }
}
