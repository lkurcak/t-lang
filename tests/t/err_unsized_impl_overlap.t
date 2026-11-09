// An `unsized` parameter can be `str`: so these two impls overlap.
use core::fmt::Display;

trait Tr { fn tr(&self) -> u8; }

impl<'a, unsized T: Display> Tr for &'a T {
    fn tr(&self) -> u8 { 0 }
}

impl<'a> Tr for &'a str {
    fn tr(&self) -> u8 { 1 }
}

fn main() {}
// error: overlap
