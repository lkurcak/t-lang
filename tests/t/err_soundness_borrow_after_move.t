// The borrow of `x` waits for the arguments, and the one after it moves
// `x`.
struct Fd { n: i64 }

fn two(a: &Fd, b: Fd) -> i64 with panic {
    let Fd { n } = b;
    a.n + n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let x = Fd { n: 1 };
    out.line(&two(&x, x))?;
    Result::Ok(())
}
// error: use of moved value `x`
