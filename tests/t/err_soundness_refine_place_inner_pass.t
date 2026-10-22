// A `&mut` that's a field, passed on, lets the callee write through it.
#[derive(Discard)]
struct W<'a> { r: &'a mut P }

#[derive(Copy, Discard)]
struct P { x: i64 }

fn zero(p: &mut P) { p.x = 0; }

fn f(w: W) -> i64 {
    if w.r.x > 0 {
        zero(w.r);
        10 / w.r.x
    } else {
        0
    }
}

fn main() with panic {
    let mut p = P { x: 1 };
    print(f(W { r: &mut p }));
}
// error: `/` may panic, but `f` doesn't say `with panic`
