enum Opt { None, Some(int) }
fn f(o: Opt) -> int {
    match o {
        Opt::Some(1 | 2) | Opt::None => 0,
    }
}
fn main() {}
// error: non-exhaustive `match`: `Opt::Some(..=0)` not covered
