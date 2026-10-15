fn dangle(n: int) -> &'static int { &n }

fn main() {}
// error: `n` doesn't live long enough: it goes away when the function returns
