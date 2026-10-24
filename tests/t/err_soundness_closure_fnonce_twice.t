// A closure that moves a captured value out can only be called once: the
// call moves the closure.
struct B { x: i64 }

fn take(b: B) -> i64 {
    let B { x } = b;
    x
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let b = B { x: 1 };
    let f = || take(b);
    out.line(&f())?;
    out.line(&f())?;
    Result::Ok(())
}
// error: use of moved value `f`
