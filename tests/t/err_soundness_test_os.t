// A test can't take the `Os`, as `main` can: tests do no I/O, so what each
// does is all in what it gives back, and none can touch another's world.
fn main() {}

#[test]
fn prints(os: std::Os) {
    let mut out = os.stdout;
    let _ = out.line(&1u8);
}
// error: a test takes nothing
