// An impl's type has a size, as a value's does: only the built-in
// libraries implement traits for `str` and slices, whose methods take care
// to have `Self` only behind a reference. Implement it for `&T` instead.
use core::fmt::Display;

trait Tr { fn tr(&self) -> u8; }

impl<unsized T: Display> Tr for T {
    fn tr(&self) -> u8 { 0 }
}

fn main() {}
// error: `T` is `unsized`, so it can only be behind a reference, as in `&T`
