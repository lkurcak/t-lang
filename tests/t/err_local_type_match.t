// Messages show local types by their names, not their symbols.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    #[derive(Discard)]
    enum Option<T> { Some(T), None }
    #[derive(Discard)]
    struct P { x: i64 }
    let o: Option<P> = Option::Some(P { x: 1 });
    match o {
        Option::Some(P { x: 2 }) => out.line(&1)?,
        Option::None => out.line(&0)?,
    }
    Result::Ok(())
}
// error: non-exhaustive `match`: `Option::Some(P { x: ..=1 })` not covered
