// The program's own `#[blame_caller]` function as a value, called through
// a parameter: shown where it became a value.
#[blame_caller]
fn positive(n: i64) -> i64 with panic {
    if n <= 0 { panic("not positive") }
    n
}

fn twice(f: fn(i64) -> i64 with panic, n: i64) -> i64 with panic { f(f(n)) }

fn main() {
    let g = positive;
    let _ = twice(g, 0);
}
// error: blame_caller_value_own.t:12:13
