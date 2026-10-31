/// Only this package can implement it: so `Describe`'s blanket impl below
/// is known not to cover any type of another package.
export sealed trait Small {}

impl Small for u8 {}

export trait Describe {
    fn describe(&self) -> i64;
}

impl<T: Small> Describe for T {
    fn describe(&self) -> i64 { 1 }
}
