use crate::Result;
use tinywasm_types::Module;

/// Parse a module from bytes. Requires `parser` feature.
pub fn parse_bytes(wasm: &[u8]) -> Result<Module> {
    let parser = tinywasm_parser::Parser::new();
    let data = parser.parse_module_bytes(wasm)?;
    Ok(data)
}
