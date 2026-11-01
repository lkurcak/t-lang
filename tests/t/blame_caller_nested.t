// A function inside another can blame its caller too.
fn main() {
    #[blame_caller]
    fn check(ok: bool) with panic { assert(ok); }
    check(true);
    check(false);
}
// error: blame_caller_nested.t:6:5
