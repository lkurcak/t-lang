// Closures: anonymous functions that can use the locals around them.

#[derive(Discard)]
struct P { x: int, y: int }

// A closure that captures has a type of its own, so a function that takes
// one is generic over it.
fn apply<F: Fn(int) -> int + Discard>(f: F, x: int) -> int { f(x) }
fn compose<F: Fn(int) -> int + Discard, G: Fn(int) -> int + Discard>(
    f: F,
    g: G,
    x: int,
) -> int {
    // A closure can capture a parameter.
    apply(|y| g(f(y)), x)
}
fn on_point<F: Fn(P) -> int + Discard>(f: F) -> int { f(P { x: 3, y: 4 }) }
fn inc(x: int) -> int { x + 1 }
// Closures that capture nothing can be returned: they're plain functions.
fn pick(double: bool) -> fn(int) -> int {
    if double { |x| x * 2 } else { inc }
}

fn main() {
    let k = 10;
    print(apply(|x| x * k, 4));             // expect: 40
    let five = 5;
    let add5 = |x| x + five;
    print(add5(1));                         // expect: 6
    print(compose(add5, |x| x + 100, 0));   // expect: 105
    // Functions work where closures do.
    print(compose(inc, |x| x * 2, 3));      // expect: 8
    print(pick(true)(pick(false)(4)));      // expect: 10
    // `p`'s type comes from `on_point`'s bound, so `p.x` works.
    print(on_point(|p| p.x * p.y));         // expect: 12
    // No parameters, annotations, a block body.
    let one = || 1;
    let both = |a: int, b| -> int { let s = a + b; s * s };
    print(one() + both(1, 2));              // expect: 10
    // A closure that borrows `n` sees the `n` it was made with, as the
    // `let` after makes a new one.
    let n = 1;
    let get = || n;
    let n = 2;
    print(get() + n);                       // expect: 3
    // Closures in data, if they capture nothing.
    let fs = Fs { a: |x| x - 1, b: pick(true) };
    print((fs.a)((fs.b)(3)));               // expect: 5
    print(apply(fs.a, 1));                  // expect: 0
    // Immediately called.
    print((|x| x + 1)(41));                 // expect: 42
    // Calling an argument with unknown type makes it a function.
    let twice = |f, x| f(f(x));
    print(twice(inc, 0));                   // expect: 2
}

#[derive(Discard)]
struct Fs { a: fn(int) -> int, b: fn(int) -> int }
