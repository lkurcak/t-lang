// A closure that moves a captured value out can only be called once: the
// call moves the closure.
struct B { x: int }

fn take(b: B) -> int {
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
