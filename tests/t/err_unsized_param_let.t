// Nor a variable's.
fn f<unsized D>(x: &D) {
    let y: D = *x;
}

fn main() {}
// error: `D` is `unsized`, so it can only be behind a reference, as in `&D`
