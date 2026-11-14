// A parameter declared `mut` can be made to point elsewhere, to what
// another parameter declares a refinement of: a closure changing `r.x`
// then captures `r`, not the field through it.
#[derive(Discard)]
struct P { x: u32, y: u32 }

fn f<'a>(mut r: &'a mut P, q: &'a mut { v: P | v.x < v.y }) {
    r = q;
    let mut c = || r.x = 5;
    r.y = 3; // error: the closure captures `r`, not just `r.x`, as `r` is declared `mut`
    c();
}

fn main() {
    let mut p = P { x: 0, y: 10 };
    let mut o = P { x: 0, y: 10 };
    f(&mut o, &mut p);
}
