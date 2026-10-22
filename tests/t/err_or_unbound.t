#[derive(Discard)]
enum Opt { None, Some(i64) }
fn f(o: Opt) -> i64 {
    match o {
        Opt::Some(x) | Opt::None => x,
    }
}
fn main() {}
// error: `x` is not bound in this alternative
