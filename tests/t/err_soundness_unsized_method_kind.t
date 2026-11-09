// An impl's method has the trait's kinds: here `U` would be sized in the
// impl, which could make values of it, but it's `unsized` in the trait,
// whose callers may make it `str`.
use core::fmt::Display;

trait Tr {
    fn m<unsized U: Display>(&self, u: &U) -> u8;
}

impl Tr for u8 {
    fn m<U: Display>(&self, _u: &U) -> u8 { 0 }
}

fn main() {}
// error: method `m` doesn't match its declaration in trait `Tr`: its `U` isn't `unsized`, where the trait's `U` is
