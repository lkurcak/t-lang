// Changed through a `&mut` the body lends, too.
fn bump(n: &mut usize) { *n = 5; }

fn twice<F: FnMut() -> u32 + Discard>(mut f: F) -> u32 {
    let _ = f();
    f()
}

fn main() {
    let a: [u32; 3] = [1, 2, 3];
    let mut n: usize = 0;
    let _ = twice(move || {
        let r = a[n];
        bump(&mut n);
        r
    });
}
// error: can't prove `i < self.len()` for argument `i` of `index`
