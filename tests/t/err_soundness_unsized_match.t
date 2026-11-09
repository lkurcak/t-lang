// Nor is it, matched and bound.
fn f<unsized D: Copy>(x: &D) {
    match *x {
        _v => {}
    }
}

fn main() {}
// error: `D` is `unsized`, so it can only be behind a reference, as in `&D`
