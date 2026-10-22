#[derive(Discard)]
enum Opt { None, Some(i64) }
fn main() {
    let Opt::Some(x) = Opt::Some(1);
}
// error: refutable pattern in `let`: `Opt::None` not covered
