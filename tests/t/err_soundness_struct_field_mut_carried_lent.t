// And lent to a call that writes anything, it's proved again after.
#[derive(Copy, Discard)]
struct S { a: { v: u8 | v < 3 }, b: u8 }

fn big(r: &mut u8) {
    *r = 200;
}

fn main() {
    let mut s = S { a: 1, b: 2 };
    let r = &mut s.a;
    big(r);
}
// error: can't prove `v < 3`
