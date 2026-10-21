// When the closure runs, `d` may not be what it was when it was made.
fn apply<F: FnMut() -> int + Discard>(mut f: F) -> int { f() }

fn main() with panic {
    let mut d = 1;
    let c = || 10 / d;
    print(apply(c));
    d = 0;
    print(d);
}
// error: this closure may panic, but `apply` needs calls of it to fit `fn() -> int`, which can't panic
