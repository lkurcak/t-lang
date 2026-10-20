// A bound with no `with` says calling the closure can't panic, so `apply`
// needn't say it may. This closure may.
fn apply<F: Fn(int) -> int + Discard>(f: F) -> int { f(1) }

fn main() with panic {
    let z = 0;
    print(apply(|x| x / z));
}
// error: this closure may panic, but `apply` needs calls of it to fit `fn(int) -> int`, which can't panic
