// An array's element bound by `&mut` through match ergonomics: what's
// written through it must keep what's declared of the array's elements.
fn main() {
    let mut o: [{ s: u8 | s < 64 }; 2] = [1, 2];
    match &mut o {
        [a, _] => *a = 99,
    }
    let _ = o;
}
// error: can't prove `s < 64` for what's written through `*a`
