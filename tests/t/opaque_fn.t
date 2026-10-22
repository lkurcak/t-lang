// A function can return a closure: its return type says only what the
// closure can do, `impl Fn(i64) -> i64`, not which closure it is.
fn adder(n: i64) -> impl Fn(i64) -> i64 with panic + Discard { move |x| x + n }

fn counter() -> impl FnMut() -> i64 with panic + Discard {
    let mut n = 0;
    move || {
        n += 1;
        n
    }
}

fn compose<e, F: Fn(i64) -> i64 with e + Discard, G: Fn(i64) -> i64 with e + Discard>(
    f: F,
    g: G,
) -> impl Fn(i64) -> i64 with e + Discard {
    move |x| g(f(x))
}

// Two calls of one function return the same type.
fn pick(c: bool) -> impl Fn(i64) -> i64 with panic + Discard {
    let a = adder(1);
    let b = adder(2);
    if c { a } else { b }
}

fn main() with panic {
    let add5 = adder(5);
    print(add5(1));                 // expect: 6
    let mut c = counter();
    print(c());                     // expect: 1
    print(c());                     // expect: 2
    let h = compose(adder(1), adder(10));
    print(h(0));                    // expect: 11
    print(pick(false)(0));          // expect: 2
    print(adder(2)(3));             // expect: 5
}
