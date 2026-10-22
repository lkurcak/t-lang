// So does reborrowing what it points to.
#[derive(Copy, Discard)]
struct P { x: i64 }

fn zero(p: &mut P) { p.x = 0; }

fn f(r: &mut P) -> i64 {
    if r.x > 0 {
        zero(&mut *r);
        10 / r.x
    } else {
        0
    }
}

fn main() with panic {
    let mut p = P { x: 1 };
    print(f(&mut p));
}
// error: `/` may panic, but `f` doesn't say `with panic`
