// A closure that moves a captured value out can only be called once: the
// call moves the closure.
struct B { x: i64 }

fn take(b: B) -> i64 {
    let B { x } = b;
    x
}

fn main() {
    let b = B { x: 1 };
    let f = || take(b);
    print(f());
    print(f());
}
// error: use of moved value `f`
