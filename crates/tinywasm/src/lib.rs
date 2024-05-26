// #![no_std]
#![doc(test(
    no_crate_inject,
    attr(deny(warnings, rust_2018_idioms), allow(dead_code, unused_assignments, unused_variables))
))]
#![allow(unexpected_cfgs, clippy::reserve_after_initialization)]
// #![warn(missing_docs, missing_debug_implementations, rust_2018_idioms, unreachable_pub)]
#![cfg_attr(nightly, feature(error_in_core))]
#![forbid(unsafe_code)]

//! A tiny WebAssembly Runtime written in Rust
//!
//! TinyWasm provides a minimal WebAssembly runtime for executing WebAssembly modules.
//! It currently supports all features of the WebAssembly MVP specification and is
//! designed to be easy to use and integrate in other projects.
//!
//! ## Features
//!- **`std`**\
//!  Enables the use of `std` and `std::io` for parsing from files and streams. This is enabled by default.
//!- **`parser`**\
//!  Enables the `tinywasm-parser` crate. This is enabled by default.
//!- **`archive`**\
//!  Enables pre-parsing of archives. This is enabled by default.
//!
//! With all these features disabled, TinyWasm only depends on `core`, `alloc` and `libm`.
//! By disabling `std`, you can use TinyWasm in `no_std` environments. This requires
//! a custom allocator and removes support for parsing from files and streams, but otherwise the API is the same.
//! Additionally, to have proper error types in `no_std`, you currently need a `nightly` compiler to use the unstable error trait in `core`.
//!
//! ## Getting Started
//! The easiest way to get started is to use the [`Module::parse_bytes`] function to load a
//! WebAssembly module from bytes. This will parse the module and validate it, returning
//! a [`Module`] that can be used to instantiate the module.
//!
//!
//! ```rust
//! use tinywasm::{Store, Module};
//!
//! // Load a module from bytes
//! let wasm = include_bytes!("../../../examples/wasm/add.wasm");
//! let module = Module::parse_bytes(wasm)?;
//!
//! // Create a new store
//! // Stores are used to allocate objects like functions and globals
//! let mut store = Store::default();
//!
//! // Instantiate the module
//! // This will allocate the module and its globals into the store
//! // and execute the module's start function.
//! // Every ModuleInstance has its own ID space for functions, globals, etc.
//! let instance = module.instantiate(&mut store, None)?;
//!
//! // Get a typed handle to the exported "add" function
//! // Alternatively, you can use `instance.get_func` to get an untyped handle
//! // that takes and returns [`WasmValue`]s
//! let func = instance.exported_func::<(i32, i32), i32>(&mut store, "add")?;
//! let res = func.call(&mut store, (1, 2))?;
//!
//! assert_eq!(res, 3);
//! # Ok::<(), tinywasm::Error>(())
//! ```
//!
//! For more examples, see the [`examples`](https://github.com/explodingcamera/tinywasm/tree/main/examples) directory.
//!
//! ## Imports
//!
//! To provide imports to a module, you can use the [`Imports`] struct.
//! This struct allows you to register custom functions, globals, memories, tables,
//! and other modules to be linked into the module when it is instantiated.
//!
//! See the [`Imports`] documentation for more information.

mod std;
extern crate alloc;

mod error;
pub use error::*;
// pub use func::{FuncHandle, FuncHandleTyped};
pub use func::*;
pub use imports::*;
pub use instance::Instance;
pub use module::parse_bytes;
pub use reference::*;
pub use tinywasm_types::Module;

mod func;
mod imports;
mod instance;
mod module;
mod reference;
mod store;

/// Runtime for executing WebAssembly modules.
pub mod runtime;

#[cfg(feature = "parser")]
/// Re-export of [`tinywasm_parser`]. Requires `parser` feature.
pub mod parser {
    pub use tinywasm_parser::*;
}

/// Re-export of [`tinywasm_types`].
pub mod types {
    pub use tinywasm_types::*;
}

#[cold]
pub(crate) fn cold() {}

pub(crate) fn unlikely(b: bool) -> bool {
    if b {
        cold()
    };
    b
}

pub(crate) trait VecExt<T> {
    fn add(&mut self, elemnt: T) -> usize;

    fn get_or<E, F>(&self, index: usize, err: F) -> Result<&T, E>
    where
        F: FnOnce() -> E;
    fn get_mut_or<E, F>(&mut self, index: usize, err: F) -> Result<&mut T, E>
    where
        F: FnOnce() -> E;

    fn get_or_instance(&self, index: u32, name: &str) -> Result<&T, Error>;
    fn get_mut_or_instance(&mut self, index: u32, name: &str) -> Result<&mut T, Error>;
}
impl<T> VecExt<T> for Vec<T> {
    fn add(&mut self, value: T) -> usize {
        self.push(value);
        self.len() - 1
    }

    fn get_or<E, F>(&self, index: usize, err: F) -> Result<&T, E>
    where
        F: FnOnce() -> E,
    {
        self.get(index).ok_or_else(err)
    }

    fn get_mut_or<E, F>(&mut self, index: usize, err: F) -> Result<&mut T, E>
    where
        F: FnOnce() -> E,
    {
        self.get_mut(index).ok_or_else(err)
    }

    fn get_or_instance(&self, index: u32, name: &str) -> Result<&T, Error> {
        self.get_or(index as usize, || Instance::not_found_error(name))
    }
    fn get_mut_or_instance(&mut self, index: u32, name: &str) -> Result<&mut T, Error> {
        self.get_mut_or(index as usize, || Instance::not_found_error(name))
    }
}
