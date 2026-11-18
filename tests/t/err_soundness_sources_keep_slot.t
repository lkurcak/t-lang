// `h` gives `f` to another closure, which keeps it: so `f` isn't walked
// knowing what `h` gives it (lemma W7 of docs/63, its keep through
// docs/83's analysis).
fn need8(_r: { r: usize | r < 8 }) {}

fn h<T, F: FnMut(&mut T), H: FnOnce(F)>(_x: &mut T, f: F, g: H) {
    g(f)
}

fn main() {
    let mut x: { v: usize | v < 8 } = 5;
    let mut slot = None;
    h(&mut x, |r: &mut usize| need8(*r), |f| slot = Some(f));
    match slot {
        Some(mut f) => {
            let mut y = 100usize;
            f(&mut y)
        }
        None => {}
    }
}
// error: can't prove `r < 8`
