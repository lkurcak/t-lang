// `f` returns what holds what `g` returns, which holds what `f` returns.
fn f(n: i64) -> impl Fn() -> i64 + Discard {
    let h = g(n);
    move || h()
}

fn g(n: i64) -> impl Fn() -> i64 + Discard {
    let h = f(n);
    move || h()
}

fn main() { print(f(1)()); }
// error: would contain itself
