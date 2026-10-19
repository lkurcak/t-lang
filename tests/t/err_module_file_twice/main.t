// The same module twice is two modules of the same name, file or not.
mod a;
mod a { } // error: `a` is already defined
fn main() {}
