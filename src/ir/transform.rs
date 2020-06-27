use crate::error::Result;

pub trait Transform<T> {
    /// Name of the transformation.
    fn name(&self) -> &'static str;

    /// Concise description of the transformation.
    fn description(&self) -> &'static str;

    /// Applies the transformation to `program`.
    fn transform(&self, program: &mut T) -> Result<()>;
}
