// Two local types of the same name are different types, and the message
// says so, since they look the same in it.
fn f() -> i64 {
    #[derive(Discard)]
    struct P { x: i64 }
    fn g(p: P) -> i64 { p.x }
    fn h() -> i64 {
        #[derive(Discard)]
        struct P { x: i64 }
        g(P { x: 1 })
    }
    h()
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f())?;
    Result::Ok(())
}
// error: expected `P`, found `P`; these are two different types called `P`, one declared in `h` and one in `f`
