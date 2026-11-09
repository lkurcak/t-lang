// And the other way: one kind, as the trait declares it.
use core::fmt::Display;

trait Tr {
    fn m<U: Display>(&self, u: &U) -> u8;
}

impl Tr for u8 {
    fn m<unsized U: Display>(&self, _u: &U) -> u8 { 0 }
}

fn main() {}
// error: method `m` doesn't match its declaration in trait `Tr`: its `U` is `unsized`, where the trait's `U` isn't
