enum Opt { None, Some(int) }
fn f(o: Opt) -> int {
    match o {
        Opt::Some(x) | Opt::None => x,
    }
}
fn main() {}
// error: `x` is not bound in this alternative
