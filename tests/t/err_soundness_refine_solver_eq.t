// `y` is `x + 1`, so `y + x` is `2 * x + 1`, which isn't always -1.
fn minus_one(n: { v: int | v == -1 }) -> int { n }

fn f(x: int) -> int with panic {
    let y = x + 1;
    minus_one(y + x)
}

fn main() with panic {
    print(f(1));
}
// error: can't prove `v == -1` for argument `n` of `minus_one`
