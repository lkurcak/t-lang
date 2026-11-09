// An alias's parameter that isn't `unsized` has a size, wherever its body
// puts it.
type Shared<'a, T> = &'a T;

fn f<'a>(s: Shared<'a, str>) {}

fn main() {}
// error: `str` can only be behind a shared reference, as in `&str`
