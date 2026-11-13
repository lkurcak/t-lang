// Each place a closure returns is a case.
fn call<R, F: FnOnce() -> R + Discard>(f: F) -> R { f() }

fn main() {
    let c = true;
    let r: { v: u32 | v > 0 } = call(|| {
        if c {
            return 0;
        }
        5
    });
    let _ = r;
}
// error: can't prove `v > 0`
