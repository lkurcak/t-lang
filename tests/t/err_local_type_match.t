// Messages show local types by their names, not their symbols.
fn main() {
    enum Option<T> { Some(T), None }
    struct P { x: int }
    let o: Option<P> = Option::Some(P { x: 1 });
    match o {
        Option::Some(P { x: 2 }) => print(1),
        Option::None => print(0),
    }
}
// error: non-exhaustive `match`: `Option::Some(P { x: ..=1 })` not covered
