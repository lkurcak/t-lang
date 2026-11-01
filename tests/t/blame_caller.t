// A panic directly in a `#[blame_caller]` function is shown where it was
// called: here, `check`'s call in `main`, not the `panic` inside it.
#[blame_caller]
fn check(ok: bool) with panic {
    if !ok { panic("not ok") }
}

fn main() {
    check(true);
    check(false);
}
// error: blame_caller.t:10:5
