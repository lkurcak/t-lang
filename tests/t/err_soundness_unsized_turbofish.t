// Nor when it's said outright.
fn copy<T: Copy>(x: &T) -> T { *x }

fn f<unsized D: Copy>(x: &D) {
    let _y = copy::<D>(x);
}

fn main() {}
// error: `D` is `unsized`, so it can only be behind a reference, as in `&D`
