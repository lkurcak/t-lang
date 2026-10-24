// A `move` closure that owns `fd` but only reads it would be callable many
// times and never give `fd` back: `fd` would be dropped with it.
struct Fd { n: i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let fd = Fd { n: 3 };
    let c = move || fd.n;
    out.line(&c())?;
    Result::Ok(())
}
// error: `fd` is never used up
