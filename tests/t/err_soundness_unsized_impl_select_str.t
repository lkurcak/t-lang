// Nor a `str`, as ever.
use core::fmt::Display;

trait Tr { fn tr(&self) -> u8; }

impl<'a, T: Display> Tr for &'a T {
    fn tr(&self) -> u8 { 0 }
}

fn main() {
    let s: &str = "abc";
    let _n = s.tr();
}
// error: `str` has no method `tr`
