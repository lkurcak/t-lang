// Only one branch makes `d` non-zero.
fn f(c: bool) -> int {
    let mut d = 0;
    if c {
        d = 1;
    }
    10 / d
}

fn main() with panic {
    print(f(false));
}
// error: `/` may panic, but `f` doesn't say `with panic`
