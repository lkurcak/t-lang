// A struct's parameter that isn't `unsized` has a size where it's used.
struct Holder<'a, T> { r: &'a T, copy: Option<T> }

fn f<'a, unsized D>(h: Holder<'a, D>) {}

fn main() {}
// error: `D` is `unsized`, so it can only be behind a reference, as in `&D`
