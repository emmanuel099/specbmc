use crate::error::Result;

pub trait TryTranslateInto<T> {
    fn try_translate_into(&self) -> Result<T>;
}

pub trait TryTranslateFrom<T> {
    type Target;
    fn try_translate_from(src: &T) -> Result<Self::Target>;
}

impl<S: TryTranslateInto<T>, T> TryTranslateFrom<S> for T {
    type Target = T;

    fn try_translate_from(src: &S) -> Result<Self> {
        src.try_translate_into()
    }
}
