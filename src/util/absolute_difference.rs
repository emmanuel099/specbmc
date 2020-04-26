use std::ops::Sub;

pub trait AbsoluteDifference<T: Sub<Output = T> + Ord> {
    fn abs_diff(self, other: T) -> T;
}

impl<T: Sub<Output = T> + Ord> AbsoluteDifference<T> for T {
    fn abs_diff(self, other: T) -> T {
        if self < other {
            other - self
        } else {
            self - other
        }
    }
}
