#[derive(Discard)]
enum Option<T> { Some(T), None }

fn main() {
    let o = Option::Some(1);
    match &o {
        Option::Some(mut n) => print(*n),
        Option::None => {}
    }
}
// error: `n` can't be `mut` here: it's bound by reference
