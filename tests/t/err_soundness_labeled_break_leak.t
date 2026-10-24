// `break 'a` out of two loops leaves `_a` behind.
struct Fd { n: i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x = 'a: loop {
        let _a = Fd { n: 10 };
        loop {
            break 'a 1i64;
        }
    };
    out.line(&x)?;
    Result::Ok(())
}
// error: `_a` is never used up, and its type, `Fd`, isn't `Discard`
