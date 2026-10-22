// What a closure's body learns is of when it runs, not of what comes after
// it's made: `d` is still 0 there.
fn non_zero(n: { v: i64 | v != 0 }) -> i64 { n }

fn pick() -> i64 { 0 }

fn main() with panic {
    let d = pick();
    let c = |x: i64| -> i64 {
        if d == 0 {
            return x;
        }
        x / d
    };
    print(c(1));
    print(non_zero(d));
}
// error: can't prove `v != 0` for argument `n` of `non_zero`
