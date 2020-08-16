use crate::error::Result;
use crate::hir::Function;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Program {
    functions: BTreeMap<u64, Function>,
}

impl Program {
    pub fn new() -> Self {
        Self {
            functions: BTreeMap::new(),
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
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for func in self.functions.values() {
            writeln!(f, "{}", func)?;
        }
        Ok(())
    }
}
