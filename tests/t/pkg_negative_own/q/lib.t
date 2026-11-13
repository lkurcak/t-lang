//! A library with a trait of its own and a blanket impl of it.
export trait Size {
    fn size(&self) -> i64;
}

impl<T: Iterator> Size for T {
    fn size(&self) -> i64 { 0 }
}
