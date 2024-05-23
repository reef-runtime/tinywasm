use alloc::{boxed::Box, format, string::ToString};
use tinywasm_types::*;

use crate::func::{FromWasmValueTuple, IntoWasmValueTuple};
use crate::{store::Store, Error, FuncHandle, FuncHandleTyped, Imports, MemoryRef, MemoryRefMut, Result};

/// An instanciated WebAssembly module
///
/// Backed by an Rc, so cloning is cheap
///
/// See <https://webassembly.github.io/spec/core/exec/runtime.html#module-instances>
// #[derive(Debug)]
// pub struct ModuleInstance(Rc<ModuleInstanceInner>);

#[allow(dead_code)]
#[derive(Debug)]
pub struct Instance {
    pub(crate) module: Module,
    pub(crate) store: Store,

    pub(crate) func_addrs: Box<[FuncAddr]>,
    pub(crate) table_addrs: Box<[TableAddr]>,
    pub(crate) mem_addrs: Box<[MemAddr]>,
    pub(crate) global_addrs: Box<[GlobalAddr]>,
    pub(crate) elem_addrs: Box<[ElemAddr]>,
    pub(crate) data_addrs: Box<[DataAddr]>,
}

impl Instance {
    /// Instantiate the module in the given store
    ///
    /// See <https://webassembly.github.io/spec/core/exec/modules.html#exec-instantiation>
    pub fn instantiate(module: Module, imports: Imports) -> Result<Self> {
        // This doesn't completely follow the steps in the spec, but the end result is the same
        // Constant expressions are evaluated directly where they are used, so we
        // don't need to create a auxiliary frame etc.

        let mut store = Store::default();

        let mut addrs = imports.link(&mut store, &module)?;

        addrs.funcs.extend(store.init_funcs(module.funcs.clone().into())?);
        addrs.tables.extend(store.init_tables(module.table_types.clone().into())?);
        addrs.memories.extend(store.init_memories(module.memory_types.clone().into())?);

        let global_addrs = store.init_globals(addrs.globals, module.globals.clone().into(), &addrs.funcs)?;
        let (elem_addrs, elem_trapped) =
            store.init_elements(&addrs.tables, &addrs.funcs, &global_addrs, &module.elements)?;
        if let Some(trap) = elem_trapped {
            return Err(Error::Trap(trap));
        }
        let (data_addrs, data_trapped) = store.init_datas(&addrs.memories, module.data.clone().into())?;
        if let Some(trap) = data_trapped {
            return Err(Error::Trap(trap));
        }

        let instance = Instance {
            module,
            store,

            // failed_to_instantiate: elem_trapped.is_some() || data_trapped.is_some(),
            // types: module.func_types,

            //
            func_addrs: addrs.funcs.into_boxed_slice(),
            table_addrs: addrs.tables.into_boxed_slice(),
            mem_addrs: addrs.memories.into_boxed_slice(),
            global_addrs: global_addrs.into_boxed_slice(),
            elem_addrs,
            data_addrs,
            // func_start: module.start_func,
            // imports: module.imports,
            // exports: module.exports,
        };

        Ok(instance)
    }

    pub fn instantiate_start(module: Module, imports: Imports, max_cycles: usize) -> Result<Self> {
        let mut instance = Self::instantiate(module, imports)?;
        let _ = instance.start(max_cycles)?;
        Ok(instance)
    }

    /// Get a export by name
    pub fn export_addr(&self, name: &str) -> Option<ExternVal> {
        let exports = self.module.exports.iter().find(|e| e.name == name.into())?;
        let addr = match exports.kind {
            ExternalKind::Func => self.func_addrs.get(exports.index as usize)?,
            ExternalKind::Table => self.table_addrs.get(exports.index as usize)?,
            ExternalKind::Memory => self.mem_addrs.get(exports.index as usize)?,
            ExternalKind::Global => self.global_addrs.get(exports.index as usize)?,
        };

        Some(ExternVal::new(exports.kind, *addr))
    }

    #[inline]
    pub(crate) fn func_ty(&self, addr: FuncAddr) -> &FuncType {
        self.module.func_types.get(addr as usize).expect("No func type for func, this is a bug")
    }

    #[inline]
    pub(crate) fn func_addrs(&self) -> &[FuncAddr] {
        &self.func_addrs
    }

    // resolve a function address to the global store address
    #[inline(always)]
    pub(crate) fn resolve_func_addr(&self, addr: FuncAddr) -> FuncAddr {
        self.func_addrs[addr as usize]
    }

    // resolve a table address to the global store address
    #[inline(always)]
    pub(crate) fn resolve_table_addr(&self, addr: TableAddr) -> TableAddr {
        self.table_addrs[addr as usize]
    }

    // resolve a memory address to the global store address
    #[inline(always)]
    pub(crate) fn resolve_mem_addr(&self, addr: MemAddr) -> MemAddr {
        self.mem_addrs[addr as usize]
    }

    // resolve a data address to the global store address
    #[inline(always)]
    pub(crate) fn resolve_data_addr(&self, addr: DataAddr) -> MemAddr {
        self.data_addrs[addr as usize]
    }

    // resolve a memory address to the global store address
    #[inline(always)]
    pub(crate) fn resolve_elem_addr(&self, addr: ElemAddr) -> ElemAddr {
        self.elem_addrs[addr as usize]
    }

    // resolve a global address to the global store address
    #[inline(always)]
    pub(crate) fn resolve_global_addr(&self, addr: GlobalAddr) -> GlobalAddr {
        self.global_addrs[addr as usize]
    }

    /// Get an exported function by name
    pub fn exported_func_untyped<'i>(&'i mut self, name: &str) -> Result<FuncHandle<'i>> {
        let export = self.export_addr(name).ok_or_else(|| Error::Other(format!("Export not found: {}", name)))?;
        let ExternVal::Func(func_addr) = export else {
            return Err(Error::Other(format!("Export is not a function: {}", name)));
        };

        let func_inst = self.store.get_func(func_addr)?;
        let ty = func_inst.func.ty();

        Ok(FuncHandle { addr: func_addr, name: Some(name.to_string()), ty: ty.clone(), instance: self })
    }

    /// Get a typed exported function by name
    pub fn exported_func<P, R>(&mut self, name: &str) -> Result<FuncHandleTyped<P, R>>
    where
        P: IntoWasmValueTuple,
        R: FromWasmValueTuple,
    {
        let func = self.exported_func_untyped(name)?;
        Ok(FuncHandleTyped { func, marker: core::marker::PhantomData })
    }

    /// Get an exported memory by name
    pub fn exported_memory<'i>(&'i self, name: &str) -> Result<MemoryRef<'i>> {
        let export = self.export_addr(name).ok_or_else(|| Error::Other(format!("Export not found: {}", name)))?;
        let ExternVal::Memory(mem_addr) = export else {
            return Err(Error::Other(format!("Export is not a memory: {}", name)));
        };

        self.memory(mem_addr)
    }

    /// Get an exported memory by name
    pub fn exported_memory_mut<'i>(&'i mut self, name: &str) -> Result<MemoryRefMut<'i>> {
        let export = self.export_addr(name).ok_or_else(|| Error::Other(format!("Export not found: {}", name)))?;
        let ExternVal::Memory(mem_addr) = export else {
            return Err(Error::Other(format!("Export is not a memory: {}", name)));
        };

        self.memory_mut(mem_addr)
    }

    /// Get a memory by address
    pub fn memory(&self, addr: MemAddr) -> Result<MemoryRef<'_>> {
        let mem = self.store.get_mem(self.resolve_mem_addr(addr))?;
        Ok(MemoryRef { instance: mem.borrow() })
    }

    /// Get a memory by address (mutable)
    pub fn memory_mut(&mut self, addr: MemAddr) -> Result<MemoryRefMut<'_>> {
        let mem = self.store.get_mem(self.resolve_mem_addr(addr))?;
        Ok(MemoryRefMut { instance: mem.borrow_mut() })
    }

    /// Get the start function of the module
    ///
    /// Returns None if the module has no start function
    /// If no start function is specified, also checks for a _start function in the exports
    ///
    /// See <https://webassembly.github.io/spec/core/syntax/modules.html#start-function>
    pub fn start_func(&mut self) -> Result<Option<FuncHandle<'_>>> {
        let func_index = match self.module.start_func {
            Some(func_index) => func_index,
            None => {
                // alternatively, check for a _start function in the exports
                let Some(ExternVal::Func(func_addr)) = self.export_addr("_start") else {
                    return Ok(None);
                };

                func_addr
            }
        };

        let func_addr = self.func_addrs.get(func_index as usize).expect("No func addr for start func, this is a bug");
        let func_inst = self.store.get_func(*func_addr)?;
        let ty = func_inst.func.ty();

        Ok(Some(FuncHandle { addr: *func_addr, ty: ty.clone(), name: None, instance: self }))
    }

    /// Invoke the start function of the module
    ///
    /// Returns None if the module has no start function
    ///
    /// See <https://webassembly.github.io/spec/core/syntax/modules.html#syntax-start>
    pub fn start(&mut self, max_cycles: usize) -> Result<Option<()>> {
        let Some(mut func) = self.start_func()? else {
            return Ok(None);
        };

        let _ = func.call(&[], None, max_cycles)?;
        Ok(Some(()))
    }
}
