// A move that is in an earlier time round the loop.
#[derive(Discard)]
struct B { n: i64 }

fn take(b: B) -> i64 { b.n }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x = B { n: 1 };
    let mut i: i64 = 0;
    while i < 2 {
        out.line(&take(x))?;
        i += 1;
    }
    Result::Ok(())
}
// error: moved here, in an earlier time round the loop
