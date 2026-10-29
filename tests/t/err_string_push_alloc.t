// Pushing may allocate.
use alloc::string::String;

fn add(s: &mut String) with {} {
    s.push('a');
}

fn main() with {alloc, panic} {
    let mut s = String::new();
    add(&mut s);
    s.discard();
}
// error: `push` may allocate, but `add` doesn't say `with alloc`
