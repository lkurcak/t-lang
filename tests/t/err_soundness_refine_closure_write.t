// The closure changes `d`, when it's called.
fn f() -> i64 {
    let mut d = 1;
    let mut c = || d = 0;
    c();
    10 / d
}

fn main() with panic {
    print(f());
}
// error: `/` may panic, but `f` doesn't say `with panic`
