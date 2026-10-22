#[derive(Discard)]
enum Opt { None, Some(i64) }
fn main() {
    let x = match Opt::None { Opt::Some(n) => n, Opt::None => false };
}
// error: `match` arms have different types: `i64` and `bool`
