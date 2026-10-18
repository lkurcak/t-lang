// Closures as values of their own types, taken by generic functions through
// the `Fn` traits.

fn apply<F: Fn(int) -> int + Discard>(f: F, x: int) -> int { f(x) }
fn twice<F: FnMut()>(mut f: F) -> F {
    f();
    f();
    f
}
fn once<F: FnOnce() -> int>(f: F) -> int { f() }
fn inc(x: int) -> int { x + 1 }

struct B { x: int }

fn take(b: B) -> int {
    let B { x } = b;
    x
}

fn main() {
    // Captures by reference: `k` is only read.
    let k = 10;
    print(apply(|x| x * k, 4));             // expect: 40
    // Functions implement the `Fn` traits too.
    print(apply(inc, 1));                   // expect: 2
    // A closure that captures nothing is a function.
    print(apply(|x| x - 1, 1));             // expect: 0
    // Changing a captured variable borrows it mutably, for as long as the
    // closure is used.
    let mut n = 0;
    let c = twice(|| n = n + 1);
    let mut c = twice(c);
    c();
    print(n);                               // expect: 5
    // Moving a captured value out makes the closure `FnOnce`.
    let b = B { x: 7 };
    print(once(|| take(b)));                // expect: 7
    // `Fn` closures are `FnOnce` too.
    print(once(|| k + 1));                  // expect: 11
    // Lending a closure: `&F` is `Fn` when `F` is.
    let add = |x| x + k;
    print(apply(&add, 1) + apply(&add, 2)); // expect: 23
}
