use crate::Result;
use tinywasm_types::Module;

#[cfg(feature = "parser")]
/// Parse a module from bytes. Requires `parser` feature.
pub fn parse_bytes(wasm: &[u8]) -> Result<Module> {
    let parser = tinywasm_parser::Parser::new();
    let data = parser.parse_module_bytes(wasm)?;
    Ok(data.into())
}

// #[cfg(all(feature = "parser", feature = "std"))]
// /// Parse a module from a stream. Requires `parser` and `std` features.
// pub fn parse_stream(stream: impl crate::std::io::Read) -> Result<Module> {
//     let parser = tinywasm_parser::Parser::new();
//     let data = parser.parse_module_stream(stream)?;
//     Ok(data.into())
// }
