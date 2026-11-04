// Nor a method: C calls it by its name alone.
struct S {}

impl S {
    extern fn f() {}
// error: an `extern fn` is a module's
}

fn main() {}
