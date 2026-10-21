// Inside the loop, `d` may be 0 on the second time round.
fn f() -> int {
    let mut d = 1;
    let mut acc = 0;
    loop {
        acc = 10 / d;
        d = 0;
        if acc == 0 {
            break;
        }
    }
    acc
}

fn main() with panic {
    print(f());
}
// error: `/` may panic, but `f` doesn't say `with panic`
