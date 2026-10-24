// Two in different blocks of the same function.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p = {
        #[derive(Discard)]
        struct P { x: i64 }
        P { x: 1 }
    };
    let q = {
        #[derive(Discard)]
        struct P { x: i64 }
        P { x: 2 }
    };
    let r = if true { p } else { q };
    out.line(&r.x)?;
    Result::Ok(())
}
// error: `if` and `else` have different types: `P` and `P`; these are two different types called `P`, declared in different blocks in `main`
