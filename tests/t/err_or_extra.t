enum Opt { None, Some(int) }
fn f(o: Opt) -> int {
    match o {
        Opt::None | Opt::Some(x) => 0,
    }
}
fn main() {}
// error: `x` is bound in this alternative but not in the first
