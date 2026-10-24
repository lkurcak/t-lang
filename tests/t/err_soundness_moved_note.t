// The move is written after the borrow, but happens first, as a call's
// borrows wait for its arguments: not in an earlier time round a loop.
#[derive(Discard)]
struct B { n: i64 }

fn two(a: &B, b: B) -> i64 with panic { a.n + b.n }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let x = B { n: 1 };
    let y = B { n: 2 };
    let c = true;
    out.line(&two(&x, if c { x } else { y }))?;
    Result::Ok(())
}
// error: `x` was moved here
