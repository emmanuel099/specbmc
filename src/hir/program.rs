use crate::error::Result;
use crate::hir::{Function, Memory};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum ProgramEntry {
    Name(String),
    Address(u64),
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Default)]
pub struct Program {
    functions: BTreeMap<u64, Function>,
    entry: Option<ProgramEntry>,
    memory: Memory,
}

impl Program {
    pub fn new() -> Self {
        Self {
            functions: BTreeMap::new(),
            entry: None,
            memory: Memory::default(),
        }
    }

    pub fn functions(&self) -> Vec<&Function> {
        self.functions.values().collect()
    }

    pub fn functions_mut(&mut self) -> Vec<&mut Function> {
        self.functions.values_mut().collect()
    }

    pub fn function_by_address(&self, address: u64) -> Option<&Function> {
        self.functions.get(&address)
    }

    pub fn function_by_name(&self, name: &str) -> Option<&Function> {
        self.functions.values().find(|f| f.name() == Some(name))
    }

    pub fn insert_function(&mut self, func: Function) -> Result<()> {
        let addr = func.address();
        if self.functions.contains_key(&addr) {
            return Err(format!("Function with address {} exists already", addr).into());
        }
        self.functions.insert(addr, func);
        Ok(())
    }

    pub fn set_entry(&mut self, entry: ProgramEntry) -> Result<()> {
        match &entry {
            ProgramEntry::Name(name) => {
                if self.function_by_name(name).is_none() {
                    return Err(format!("Entry function '{}' does not exist", name).into());
                }
            }
            ProgramEntry::Address(addr) => {
                if self.function_by_address(*addr).is_none() {
                    return Err(
                        format!("Entry function for address {} does not exist", *addr).into(),
                    );
                }
            }
        }

        self.entry = Some(entry);
        Ok(())
    }

    pub fn entry(&self) -> &Option<ProgramEntry> {
        &self.entry
    }

    pub fn entry_function(&self) -> Option<&Function> {
        if let Some(entry) = &self.entry {
            match entry {
                ProgramEntry::Name(name) => self.function_by_name(name),
                ProgramEntry::Address(addr) => self.function_by_address(*addr),
            }
        } else {
            None
        }
    }

    pub fn memory(&self) -> &Memory {
        &self.memory
    }

    pub fn memory_mut(&mut self) -> &mut Memory {
        &mut self.memory
    }
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for func in self.functions.values() {
            writeln!(f, "{}", func)?;
        }
        Ok(())
    }
}
