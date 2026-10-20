// A struct's type parameter bounded by `Fn` with no `with` holds only
// closures that can't panic.
struct Holder<F: Fn() -> int> { f: F }

fn call<F: Fn() -> int>(h: &Holder<F>) -> int { (h.f)() }

fn main() with panic {
    let h = Holder { f: || panic() };
    print(call(&h));
}
// error: this closure may panic, but `Holder` needs calls of it to fit `fn() -> int`, which can't panic
