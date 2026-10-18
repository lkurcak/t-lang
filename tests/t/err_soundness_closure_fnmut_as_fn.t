// A closure that changes a captured variable is `FnMut`, not `Fn`: calling
// it through a shared reference could change `n` while someone reads it.
fn call<F: Fn() + Discard>(f: F) { f() }

fn main() {
    let mut n = 0;
    call(|| n = n + 1);
    print(n);
}
// error: this closure is only `FnMut`, as it changes `n`, but `call` needs it to implement `Fn`
