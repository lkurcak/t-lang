// Nor does passing it where one is wanted.
struct Fd { n: i64 }

fn run(k: fn() -> !) -> i64 { k() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let _fd = Fd { n: 1 };
    out.line(&run(|| return 1))?;
    Result::Ok(())
}
// error: expected `!`, found `{integer}`
