// A closure that moves a captured value out is only `FnOnce`, so it can't
// be passed where it would be called more than once.
struct B { x: i64 }

fn take(b: B) -> i64 {
    let B { x } = b;
    x
}

fn twice<F: Fn() -> i64 + Discard>(f: F) -> i64 with panic { f() + f() }

fn main() {
    let b = B { x: 1 };
    print(twice(|| take(b)));
}
// error: this closure is only `FnOnce`, as it moves `b` out, but `twice` needs it to implement `Fn`
