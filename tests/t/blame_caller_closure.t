// A closure's body is its own: a panic in one is shown there, even when a
// `#[blame_caller]` function calls it.
#[blame_caller]
fn apply<F: Fn(u8) -> u8 with panic + Discard>(f: F, n: u8) -> u8 with panic { f(n) }

fn main() {
    let _ = apply(|n| n + 250, 10);
}
// error: blame_caller_closure.t:7:23
