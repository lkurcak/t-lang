// A pattern can look through a `&mut` inside the struct it matches, and
// bind what's behind it by `&mut`.
#[derive(Discard)]
struct W<'a> { r: &'a mut P }

#[derive(Copy, Discard)]
struct P { x: i64 }

fn f(w: W) -> i64 {
    if w.r.x > 0 {
        let W { r: P { x } } = w;
        *x = 0;
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
