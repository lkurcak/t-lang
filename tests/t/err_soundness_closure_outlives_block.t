// A closure that borrows `n` can't outlive `n`: its type says what it
// borrows, so the borrow checker sees the reference inside it.
#[derive(Discard)]
struct Holder<F> { f: F }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let h = {
        let n: i64 = 5;
        Holder { f: || n }
    };
    out.line(&(h.f)())?;
    Result::Ok(())
}
// error: `n` doesn't live long enough
