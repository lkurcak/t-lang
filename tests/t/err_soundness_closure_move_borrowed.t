// `c` borrows `b` mutably until its last call, so `b` can't be moved
// before that.
struct B { x: i64 }

fn take(b: B) -> i64 {
    let B { x } = b;
    x
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut b = B { x: 1 };
    let mut c = || b.x = 2;
    let y = take(b);
    c();
    out.line(&y)?;
    Result::Ok(())
}
// error: cannot move out of `b` while `b` is mutably borrowed
