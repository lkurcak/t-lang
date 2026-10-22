#[derive(Discard)]
enum Opt { None, Some(i64) }
fn f(o: Opt) -> i64 {
    match o { Opt::None => 0, Opt::Some(..=-1) => 1 }
}
fn main() {}
// error: non-exhaustive `match`: `Opt::Some(0..)` not covered
