use crate::error::Result;
use std::fmt;
use std::fs::File;
use std::io::Write;
use std::path::Path;

mod compact_iterator;

pub use compact_iterator::CompactIterator;

pub trait Validate {
    fn validate(&self) -> Result<()>;
}

pub trait Transform<T> {
    /// Name of the transformation.
    fn name(&self) -> &'static str;

    /// Concise description of the transformation.
    fn description(&self) -> &'static str;

    /// Applies the transformation to `program`.
    fn transform(&self, program: &mut T) -> Result<()>;
}

pub trait TranslateInto<T> {
    fn translate_into(&self) -> Result<T>;
}

pub trait RenderGraph {
    fn render_to_str(&self) -> String;

    fn render_to_file(&self, path: &Path) -> Result<()> {
        let mut file = File::create(path)?;
        file.write_all(self.render_to_str().as_bytes())?;
        file.flush()?;
        Ok(())
    }
}

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
