// A closure given straight to a call is walked at the call, after the
// arguments after it: what it captured by value is what it was made with,
// not what's there by then (since 0.84.0 it read `n` as 1, and ran
// `a[7]`).
fn call2<F: Fn() -> u32 + Discard>(f: F, _x: ()) -> u32 { f() }

fn main() {
    let a: [u32; 3] = [1, 2, 3];
    let mut n: usize = 7;
    let _ = call2(move || a[n], {
        n = 1;
    });
}
// error: can't prove `i < self.len()` for argument `i` of `index`
