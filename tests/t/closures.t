// Closures: anonymous functions that can use the locals around them.

struct P { x: int, y: int }

fn apply(f: fn(int) -> int, x: int) -> int { f(x) }
fn compose(f: fn(int) -> int, g: fn(int) -> int) -> fn(int) -> int {
    |x| g(f(x))
}
// Returning a closure that captured a parameter: `n` outlives the call that
// bound it.
fn adder(n: int) -> fn(int) -> int { |x| x + n }
fn on_point(f: fn(P) -> int) -> int { f(P { x: 3, y: 4 }) }
fn inc(x: int) -> int { x + 1 }

fn main() {
    let k = 10;
    print(apply(|x| x * k, 4));             // expect: 40
    let add5 = adder(5);
    print(add5(1));                         // expect: 6
    print(compose(add5, adder(100))(0));    // expect: 105
    // Top-level functions and closures have the same types.
    print(compose(inc, |x| x * 2)(3));      // expect: 8
    // `p`'s type comes from `on_point`'s signature, so `p.x` works.
    print(on_point(|p| p.x * p.y));         // expect: 12
    // No parameters, annotations, a block body.
    let one = || 1;
    let both = |a: int, b| -> int { let s = a + b; s * s };
    print(one() + both(1, 2));              // expect: 10
    // Captured values are copied when the closure is made, and nothing can
    // change them afterwards.
    let n = 1;
    let get = || n;
    let n = 2;
    print(get() + n);                       // expect: 3
    // Closures in data.
    let fs = Fs { a: |x| x - 1, b: add5 };
    print((fs.a)(fs.b(0)));                 // expect: 4
    // Immediately called.
    print((|x| x + 1)(41));                 // expect: 42
    // Calling an argument with unknown type makes it a function.
    let twice = |f, x| f(f(x));
    print(twice(add5, 0));                  // expect: 10
}

struct Fs { a: fn(int) -> int, b: fn(int) -> int }
