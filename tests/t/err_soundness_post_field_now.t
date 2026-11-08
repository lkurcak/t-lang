// What a call ensures of a field behind a call-scoped `&mut` is of what it
// left there, not of what was there before.
#[derive(Discard)]
struct C { n: u8 }

fn clear(c: &mut C) ensures c.n == 0 {
    c.n = 0;
}

fn main() {
    let mut c = C { n: 5 };
    clear(&mut c);
    let k: { k: u8 | k == 5 } = c.n;
    let _ = k;
}
// error: can't prove `k == 5` for `k`
