use crate::{conversion, ParseError, Result};
use alloc::{boxed::Box, format, vec::Vec};
use tinywasm_types::{Data, Element, Export, FuncType, Global, Import, Instruction, MemoryType, TableType, ValType};
use wasmparser::{FuncValidatorAllocations, Payload, Validator};

pub(crate) type Code = (Box<[Instruction]>, Box<[ValType]>);

#[derive(Default)]
pub(crate) struct ModuleReader {
    func_validator_allocations: Option<FuncValidatorAllocations>,

    pub(crate) version: Option<u16>,
    pub(crate) start_func: Option<u32>,
    pub(crate) func_types: Vec<FuncType>,
    pub(crate) code_type_addrs: Vec<u32>,
    pub(crate) exports: Vec<Export>,
    pub(crate) code: Vec<Code>,
    pub(crate) globals: Vec<Global>,
    pub(crate) table_types: Vec<TableType>,
    pub(crate) memory_types: Vec<MemoryType>,
    pub(crate) imports: Vec<Import>,
    pub(crate) data: Vec<Data>,
    pub(crate) elements: Vec<Element>,
    pub(crate) end_reached: bool,
}

impl ModuleReader {
    pub(crate) fn new() -> ModuleReader {
        Self::default()
    }

    pub(crate) fn process_payload(&mut self, payload: Payload<'_>, validator: &mut Validator) -> Result<()> {
        use wasmparser::Payload::*;

        match payload {
            Version { num, encoding, range } => {
                validator.version(num, encoding, &range)?;
                self.version = Some(num);
                match encoding {
                    wasmparser::Encoding::Module => {}
                    wasmparser::Encoding::Component => return Err(ParseError::InvalidEncoding(encoding)),
                }
            }
            StartSection { func, range } => {
                if self.start_func.is_some() {
                    return Err(ParseError::DuplicateSection("Start section".into()));
                }

                validator.start_section(func, &range)?;
                self.start_func = Some(func);
            }
            TypeSection(reader) => {
                if !self.func_types.is_empty() {
                    return Err(ParseError::DuplicateSection("Type section".into()));
                }

                validator.type_section(&reader)?;
                self.func_types = reader
                    .into_iter()
                    .map(|t| conversion::convert_module_type(t?))
                    .collect::<Result<Vec<FuncType>>>()?;
            }

            GlobalSection(reader) => {
                if !self.globals.is_empty() {
                    return Err(ParseError::DuplicateSection("Global section".into()));
                }

                validator.global_section(&reader)?;
                self.globals = conversion::convert_module_globals(reader)?;
            }
            TableSection(reader) => {
                if !self.table_types.is_empty() {
                    return Err(ParseError::DuplicateSection("Table section".into()));
                }

                validator.table_section(&reader)?;
                self.table_types = conversion::convert_module_tables(reader)?;
            }
            MemorySection(reader) => {
                if !self.memory_types.is_empty() {
                    return Err(ParseError::DuplicateSection("Memory section".into()));
                }

                validator.memory_section(&reader)?;
                self.memory_types = conversion::convert_module_memories(reader)?;
            }
            ElementSection(reader) => {
                validator.element_section(&reader)?;
                self.elements = conversion::convert_module_elements(reader)?;
            }
            DataSection(reader) => {
                if !self.data.is_empty() {
                    return Err(ParseError::DuplicateSection("Data section".into()));
                }

                validator.data_section(&reader)?;
                self.data = conversion::convert_module_data_sections(reader)?;
            }
            DataCountSection { count, range } => {
                if !self.data.is_empty() {
                    return Err(ParseError::DuplicateSection("Data count section".into()));
                }
                validator.data_count_section(count, &range)?;
            }
            FunctionSection(reader) => {
                if !self.code_type_addrs.is_empty() {
                    return Err(ParseError::DuplicateSection("Function section".into()));
                }

                validator.function_section(&reader)?;
                self.code_type_addrs = reader.into_iter().map(|f| Ok(f?)).collect::<Result<Vec<_>>>()?;
            }
            CodeSectionStart { count, range, .. } => {
                if !self.code.is_empty() {
                    return Err(ParseError::DuplicateSection("Code section".into()));
                }

                self.code.reserve(count as usize);
                validator.code_section_start(count, &range)?;
            }
            CodeSectionEntry(function) => {
                let v = validator.code_section_entry(&function)?;
                let mut func_validator = v.into_validator(self.func_validator_allocations.take().unwrap_or_default());
                self.code.push(conversion::convert_module_code(function, &mut func_validator)?);
                self.func_validator_allocations = Some(func_validator.into_allocations());
            }
            ImportSection(reader) => {
                if !self.imports.is_empty() {
                    return Err(ParseError::DuplicateSection("Import section".into()));
                }

                validator.import_section(&reader)?;
                self.imports = conversion::convert_module_imports(reader)?;
            }
            ExportSection(reader) => {
                if !self.exports.is_empty() {
                    return Err(ParseError::DuplicateSection("Export section".into()));
                }

                validator.export_section(&reader)?;
                self.exports =
                    reader.into_iter().map(|e| conversion::convert_module_export(e?)).collect::<Result<Vec<_>>>()?;
            }
            End(offset) => {
                if self.end_reached {
                    return Err(ParseError::DuplicateSection("End section".into()));
                }

                validator.end(offset)?;
                self.end_reached = true;
            }
            CustomSection(_reader) => {
                // debug!("Skipping custom section: {:?}", _reader.name());
            }
            UnknownSection { .. } => return Err(ParseError::UnsupportedSection("Unknown section".into())),
            section => return Err(ParseError::UnsupportedSection(format!("Unsupported section: {:?}", section))),
        };

        Ok(())
    }
}
