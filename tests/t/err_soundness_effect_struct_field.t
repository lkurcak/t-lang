// A struct's type parameter bounded by `Fn` with no `with` holds only
// closures that can't panic.
struct Holder<F: Fn() -> i64> { f: F }

fn call<F: Fn() -> i64>(h: &Holder<F>) -> i64 { (h.f)() }

fn main() with panic {
    let h = Holder { f: || panic("stop") };
    print(call(&h));
}
// error: this closure may panic, but `Holder` needs calls of it to fit `fn() -> i64`, which can't panic
