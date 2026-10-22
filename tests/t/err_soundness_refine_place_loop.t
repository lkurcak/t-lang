// The second time round, the field has the value the first gave it.
#[derive(Copy, Discard)]
struct P { x: i64 }

fn f(mut p: P) -> i64 {
    let mut n = 0;
    if p.x > 0 {
        while n < 2 {
            n = 10 / p.x;
            p.x = 0;
        }
    }
    n
}

fn main() with panic {
    print(f(P { x: 1 }));
}
// error: `/` may panic, but `f` doesn't say `with panic`
