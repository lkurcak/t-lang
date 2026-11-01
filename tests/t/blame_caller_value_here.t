// Through a value, the site is where the value was made, and blame goes no
// further out, even when what calls through it blames its own caller.
#[blame_caller]
fn positive(n: i64) -> i64 with panic {
    if n <= 0 { panic("not positive") }
    n
}

#[blame_caller]
fn apply(f: fn(i64) -> i64 with panic, n: i64) -> i64 with panic { f(n) }

fn main() {
    let g = positive;
    let _ = apply(g, 1);
    let _ = apply(g, 0);
}
// error: blame_caller_value_here.t:13:13
