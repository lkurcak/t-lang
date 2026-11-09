// Swapping what two references point to swaps values: two slices of
// different lengths can't be, nor two `D`s.
fn f<unsized D>(a: &mut D, b: &mut D) {
    core::mem::swap(a, b);
}

fn main() {}
// error: a type parameter that isn't `unsized` can't stand for `D`, which is
