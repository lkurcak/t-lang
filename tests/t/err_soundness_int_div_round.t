// `/` rounds towards zero, not down: -1 / 2 is 0, not -1.
fn f(a: { v: i64 | v == -1 }) -> { r: i64 | r == -1 } { a / 2 }

fn main() {
    print(f(-1));
}
// error: can't prove `r == -1` for the result of `f`
