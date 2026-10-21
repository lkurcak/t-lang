// Nothing says `k` is positive, so the call can't be checked.
fn div(a: int, d: { d: int | d > 0 }) -> int { a / d }

fn pick() -> int { 3 }

fn main() {
    let k = pick();
    print(div(1, k));
}
// error: can't prove `d > 0` for argument `d` of `div`
