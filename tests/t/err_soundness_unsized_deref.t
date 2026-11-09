// What a reference to a `D` points to is no value, even if `D` is `Copy`:
// at `D = str` it would be one with no size.
fn f<unsized D: Copy>(x: &D) {
    let _y = *x;
}

fn main() {}
// error: `D` is `unsized`, so it can only be behind a reference, as in `&D`
