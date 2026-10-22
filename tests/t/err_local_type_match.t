// Messages show local types by their names, not their symbols.
fn main() {
    #[derive(Discard)]
    enum Option<T> { Some(T), None }
    #[derive(Discard)]
    struct P { x: i64 }
    let o: Option<P> = Option::Some(P { x: 1 });
    match o {
        Option::Some(P { x: 2 }) => print(1),
        Option::None => print(0),
    }
}
// error: non-exhaustive `match`: `Option::Some(P { x: ..=1 })` not covered
