// An array's element bound by `&mut` through match ergonomics: the same.
fn main() {
    let mut o: [{ s: u8 | s < 64 }; 2] = [1, 2];
    match &mut o {
        [a, _] => *a = 99,
    }
    let _ = o;
}
// error: can't bind `a` by `&mut`: what it's in has a refinement, which a write through the reference could break
