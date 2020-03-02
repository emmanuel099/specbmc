use crate::error::Result;

pub trait Validate {
    fn validate(&self) -> Result<()>;
}

pub trait Transform<T> {
    /// Concise description of the transformation.
    fn description(&self) -> &'static str;

    /// Applies the transformation to `program`.
    fn transform(&self, program: &mut T) -> Result<()>;
}
