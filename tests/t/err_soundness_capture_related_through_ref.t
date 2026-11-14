// Through a parameter, a field a refinement relates to another is still
// captured with the rest: were only `r.lo` lent to the closure, `r.hi`
// could be lowered under it, and `r.lo = 5` would be checked against the
// `hi` it was made with. The closure captures `r` uniquely.
#[derive(Discard)]
struct R { lo: u32, hi: { h: u32 | h >= lo } }

fn f(r: &mut R) {
    let mut c = || r.lo = 5;
    r.hi = 3; // error: the closure captures `r`, not just `r.lo`, as a refinement of `R` relates `lo` to `hi`
    c();
}

fn main() {
    let mut r = R { lo: 0, hi: 10 };
    f(&mut r);
}
