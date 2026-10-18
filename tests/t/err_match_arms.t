#[derive(Discard)]
enum Opt { None, Some(int) }
fn main() {
    let x = match Opt::None { Opt::Some(n) => n, Opt::None => false };
}
// error: `match` arms have different types: `int` and `bool`
