// A pattern that always matches makes the `else` dead code.
#[derive(Discard, Copy)]
struct Point { x: i64, y: i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p = Point { x: 1, y: 2 };
    let Point { x, y } = p else { return Result::Ok(()) };
    out.line(&(x + y))?; // expect: 3
    if let Point { x, .. } = p {
        out.line(&x)?; // expect: 1
    }
    Result::Ok(())
}
// warning: this pattern always matches, so the `else` never runs: use `let`
// warning: this pattern always matches, so this `if let` always runs its block: use `let`
