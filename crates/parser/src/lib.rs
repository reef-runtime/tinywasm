#![no_std]
#![doc(test(
    no_crate_inject,
    attr(deny(warnings, rust_2018_idioms), allow(dead_code, unused_assignments, unused_variables))
))]
#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms, unreachable_pub)]
#![forbid(unsafe_code)]
#![cfg_attr(not(feature = "std"), feature(error_in_core))]
//! See [`tinywasm`](https://docs.rs/tinywasm) for documentation.

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod conversion;
mod error;
mod module;
mod visit;
use alloc::{string::ToString, vec::Vec};
pub use error::*;
use module::ModuleReader;
use tinywasm_types::WasmFunction;
use wasmparser::{Validator, WasmFeaturesInflated};

pub use tinywasm_types::TinyWasmModule;

/// A WebAssembly parser
#[derive(Default, Debug)]
pub struct Parser {}

impl Parser {
    /// Create a new parser instance
    pub fn new() -> Self {
        Self {}
    }

    fn create_validator(&self) -> Validator {
        let features = WasmFeaturesInflated {
            bulk_memory: true,
            floats: true,
            multi_value: true,
            mutable_global: true,
            reference_types: true,
            sign_extension: true,
            saturating_float_to_int: true,

            function_references: false,
            component_model: false,
            component_model_nested_names: false,
            component_model_values: false,
            exceptions: false,
            extended_const: false,
            gc: false,
            memory64: false,
            memory_control: false,
            relaxed_simd: false,
            simd: false,
            tail_call: false,
            threads: false,
            multi_memory: false, // should be working mostly
            custom_page_sizes: false,
            shared_everything_threads: false,
        };
        Validator::new_with_features(features.into())
    }

    /// Parse a [`TinyWasmModule`] from bytes
    pub fn parse_module_bytes(&self, wasm: impl AsRef<[u8]>) -> Result<TinyWasmModule> {
        let wasm = wasm.as_ref();
        let mut validator = self.create_validator();
        let mut reader = ModuleReader::new();

        for payload in wasmparser::Parser::new(0).parse_all(wasm) {
            reader.process_payload(payload?, &mut validator)?;
        }

        if !reader.end_reached {
            return Err(ParseError::EndNotReached);
        }

        reader.try_into()
    }

    #[cfg(feature = "std")]
    /// Parse a [`TinyWasmModule`] from a file. Requires `std` feature.
    pub fn parse_module_file(&self, path: impl AsRef<crate::std::path::Path> + Clone) -> Result<TinyWasmModule> {
        use alloc::format;
        let f = crate::std::fs::File::open(path.clone())
            .map_err(|e| ParseError::Other(format!("Error opening file {:?}: {}", path.as_ref(), e)))?;

        let mut reader = crate::std::io::BufReader::new(f);
        self.parse_module_stream(&mut reader)
    }

    #[cfg(feature = "std")]
    /// Parse a [`TinyWasmModule`] from a stream. Requires `std` feature.
    pub fn parse_module_stream(&self, mut stream: impl std::io::Read) -> Result<TinyWasmModule> {
        use alloc::format;

        let mut validator = self.create_validator();
        let mut reader = ModuleReader::new();
        let mut buffer = Vec::new();
        let mut parser = wasmparser::Parser::new(0);
        let mut eof = false;

        loop {
            match parser.parse(&buffer, eof)? {
                wasmparser::Chunk::NeedMoreData(hint) => {
                    let len = buffer.len();
                    buffer.extend((0..hint).map(|_| 0u8));
                    let read_bytes = stream
                        .read(&mut buffer[len..])
                        .map_err(|e| ParseError::Other(format!("Error reading from stream: {}", e)))?;
                    buffer.truncate(len + read_bytes);
                    eof = read_bytes == 0;
                }
                wasmparser::Chunk::Parsed { consumed, payload } => {
                    reader.process_payload(payload, &mut validator)?;
                    buffer.drain(..consumed);
                    if eof || reader.end_reached {
                        return reader.try_into();
                    }
                }
            };
        }
    }
}

impl TryFrom<ModuleReader> for TinyWasmModule {
    type Error = ParseError;

    fn try_from(reader: ModuleReader) -> Result<Self> {
        if !reader.end_reached {
            return Err(ParseError::EndNotReached);
        }

        let code_type_addrs = reader.code_type_addrs;
        let local_function_count = reader.code.len();

        if code_type_addrs.len() != local_function_count {
            return Err(ParseError::Other("Code and code type address count mismatch".to_string()));
        }

        let funcs = reader
            .code
            .into_iter()
            .zip(code_type_addrs)
            .map(|((instructions, locals), ty_idx)| WasmFunction {
                instructions,
                locals,
                ty: reader.func_types.get(ty_idx as usize).expect("No func type for func, this is a bug").clone(),
            })
            .collect::<Vec<_>>();

        let globals = reader.globals;
        let table_types = reader.table_types;

        Ok(TinyWasmModule {
            funcs: funcs.into_boxed_slice(),
            func_types: reader.func_types.into_boxed_slice(),
            globals: globals.into_boxed_slice(),
            table_types: table_types.into_boxed_slice(),
            imports: reader.imports.into_boxed_slice(),
            start_func: reader.start_func,
            data: reader.data.into_boxed_slice(),
            exports: reader.exports.into_boxed_slice(),
            elements: reader.elements.into_boxed_slice(),
            memory_types: reader.memory_types.into_boxed_slice(),
        })
    }
}
