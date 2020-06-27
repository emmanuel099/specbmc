use crate::error::Result;
use std::fmt;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub trait DumpToFile {
    fn dump_to_file(&self, path: &Path) -> Result<()>;
}

impl<T: fmt::Display> DumpToFile for T {
    fn dump_to_file(&self, path: &Path) -> Result<()> {
        let mut file = File::create(path)?;
        write!(file, "{}", self)?;
        file.flush()?;
        Ok(())
    }
}
