use crate::error::Result;
use crate::hir;
use std::ffi::OsStr;
use std::fmt;
use std::path::Path;

mod falcon;
mod muasm;

pub trait Loader {
    fn assembly_info(&self) -> Result<AssemblyInfo>;
    fn load_program(&self) -> Result<hir::Program>;
}

pub fn loader_for_file(file_path: &Path) -> Option<Box<dyn Loader>> {
    match file_path.extension().and_then(OsStr::to_str) {
        Some("muasm") => Some(Box::new(muasm::MuasmLoader::new(file_path))),
        _ => Some(Box::new(falcon::FalconLoader::new(file_path))),
    }
}

pub struct FunctionInfo {
    pub address: u64,
    pub name: Option<String>,
}

impl fmt::Display for FunctionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:X}", self.address)?;
        if let Some(name) = &self.name {
            write!(f, ": {}", name)?;
        }
        Ok(())
    }
}

pub struct MemorySectionInfo {
    pub start_address: u64,
    pub end_address: u64,
}

impl fmt::Display for MemorySectionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:X} - 0x{:X}", self.start_address, self.end_address)
    }
}

pub struct AssemblyInfo {
    pub entry: u64,
    pub functions: Vec<FunctionInfo>,
    pub memory_sections: Vec<MemorySectionInfo>,
}

impl fmt::Display for AssemblyInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Entry: 0x{:X}", self.entry)?;
        writeln!(f, "Functions:")?;
        for func in &self.functions {
            writeln!(f, "  {}", func)?;
        }
        if !self.memory_sections.is_empty() {
            writeln!(f, "Memory Sections:")?;
            for section in &self.memory_sections {
                writeln!(f, "  {}", section)?;
            }
        }
        Ok(())
    }
}
