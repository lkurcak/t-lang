// Every equality bound must hold, the ones that decide a parameter too:
// `U` is `int`, from what `f` takes, and `f` gives a `bool`.
trait Go {
    fn go(&self) -> int;
}

#[derive(Discard where F: Discard)]
struct W<F> { f: F }

impl<U, F: Fn(U) -> U> Go for W<F> {
    fn go(&self) -> int { 1 }
}

fn not_zero(x: int) -> bool { x != 0 }

fn main() {
    let w = W { f: not_zero };
    print(w.go());
}
// error: `W<fn(int) -> bool>` doesn't implement `Go`
