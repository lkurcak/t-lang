// `Debug` and `Display` both have a method `fmt`: where both are in scope
// and a type has both, a path says which.
use core::fmt::{Debug, Display};

fn main() {
    let mut s = alloc::string::String::new();
    let _ = 5i32.fmt(&mut s);
}
// error: method `fmt` of this value is ambiguous: traits `core::fmt::Debug` and `core::fmt::Display` both have one
