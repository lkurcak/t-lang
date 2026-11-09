// As `err_soundness_unsized_overlap_sized.t`, the impls the other way
// round: the overlap is found whichever impl's parameter meets the other's.
use core::fmt::{Debug, Display};

trait Tr { fn tr(&self) -> u8; }

impl<'a, unsized U: Debug> Tr for &'a U {
    fn tr(&self) -> u8 { 1 }
}

impl<'a, T: Display> Tr for &'a T {
    fn tr(&self) -> u8 { 0 }
}

fn main() {}
// error: this impl of `Tr` overlaps `impl Tr for &'a U`
