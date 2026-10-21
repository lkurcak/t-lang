// A `match` on a struct holding a `&mut` can bind what's behind it by
// `&mut`, and write through that.
#[derive(Discard)]
struct W<'a> { r: &'a mut P }

#[derive(Copy, Discard)]
struct P { x: int }

fn f(w: W) -> int {
    if w.r.x > 0 {
        match w {
            W { r: P { x } } => *x = 0,
        }
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
