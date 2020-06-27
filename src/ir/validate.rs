use crate::error::Result;

pub trait Validate {
    fn validate(&self) -> Result<()>;
}
