// The inner closure changes `n`, so the outer one, which makes it, does
// too, through what it captured: it's `FnMut`, not `Fn`.
fn call<F: Fn() + Discard>(f: F) { f() }

fn main() {
    let mut n = 0;
    call(|| {
        let mut g = || n = n + 1;
        g();
    });
    print(n);
}
// error: this closure is only `FnMut`, as it changes `n`, but `call` needs it to implement `Fn`
