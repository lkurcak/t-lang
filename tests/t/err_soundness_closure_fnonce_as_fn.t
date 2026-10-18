// A closure that moves a captured value out is only `FnOnce`, so it can't
// be passed where it would be called more than once.
struct B { x: int }

fn take(b: B) -> int {
    let B { x } = b;
    x
}

fn twice<F: Fn() -> int + Discard>(f: F) -> int { f() + f() }

fn main() {
    let b = B { x: 1 };
    print(twice(|| take(b)));
}
// error: this closure is only `FnOnce`, as it moves `b` out, but `twice` needs it to implement `Fn`
