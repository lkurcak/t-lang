// The closure borrows `n`, which is gone once `bad` returns. The type it
// returns can only borrow for lifetimes `bad` is given, and it has none.
fn bad() -> impl Fn() -> int + Discard {
    let n = 1;
    || n
}

fn main() { print(bad()()); }
// error: `n` doesn't live long enough: it goes away when the function returns
