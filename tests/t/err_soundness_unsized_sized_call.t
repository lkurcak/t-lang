// A type parameter that isn't `unsized` has a size, and its function may
// make values of it: so it can't stand for an `unsized` one. At `D = str`,
// `copy` would give a `str`, by value.
fn copy<T: Copy>(x: &T) -> T { *x }

fn f<unsized D: Copy>(x: &D) {
    let _y = copy(x);
}

fn main() {}
// error: a type parameter that isn't `unsized` can't stand for `D`, which is
