// `apply` says it has its closure's effects, so passing one that may panic
// makes the call one that may panic.
fn apply<effect e, F: Fn(i64) -> i64 with e + Discard>(f: F) -> i64 with e { f(1) }

fn main() {
    print(apply(|x| x + 1));
}
// error: `apply` may panic, but `main` doesn't say `with panic`
