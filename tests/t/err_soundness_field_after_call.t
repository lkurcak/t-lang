// What a call ensures of a field is of the value it left: one assigned
// after is another.
#[derive(Discard)]
struct C { n: u8 }

fn set(c: &mut C) ensures c.n == 3 {
    c.n = 3;
}

fn main() {
    let mut c = C { n: 0 };
    set(&mut c);
    c.n = 4;
    let k: { k: u8 | k == 3 } = c.n;
    let _ = k;
}
// error: can't prove `k == 3` for `k`
