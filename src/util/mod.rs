use crate::error::Result;

pub trait SelfCompose {
    fn self_compose(&self, composition: usize) -> Self;
}

pub trait Validate {
    fn validate(&self) -> Result<()>;
}

pub trait Transform<T> {
    fn transform(&self, program: &mut T) -> Result<()>;
}
