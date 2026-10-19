// `f` returns what holds what `g` returns, which holds what `f` returns.
fn f(n: int) -> impl Fn() -> int + Discard {
    let h = g(n);
    move || h()
}

fn g(n: int) -> impl Fn() -> int + Discard {
    let h = f(n);
    move || h()
}

fn main() { print(f(1)()); }
// error: would contain itself
