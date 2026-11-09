// A closure's parameter is a value too.
fn f<unsized D>(x: &D) {
    let c = |v: D| 0u8;
}

fn main() {}
// error: `D` is `unsized`, so it can only be behind a reference, as in `&D`
