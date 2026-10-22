// A function can call itself, getting its own opaque type, as long as the
// type behind it doesn't hold that.
fn count(n: i64) -> impl Fn() -> i64 with panic + Discard with panic {
    let k = if n > 0 { count(n - 1)() + 1 } else { 0 };
    move || k
}

// What the bounds say callers can do: copy it, or compare it.
fn adder(n: i64) -> impl Fn(i64) -> i64 with panic + Copy { move |x| x + n }

fn one() -> impl Eq + Discard { 1i64 }

// The type behind it can be a type parameter, which has what it needs by
// the function's bounds.
trait Show {
    fn show(&self) -> i64 with panic;
}

impl Show for i64 {
    fn show(&self) -> i64 with panic { *self * 10 }
}

fn same<T: Show + Discard>(x: T) -> impl Show + Discard { x }

fn main() with panic {
    print(count(3)());              // expect: 3
    let f = adder(1);
    let g = f;
    print(f(1) + g(2));             // expect: 5
    print_bool(one() == one());     // expect: true
    print(same(4i64).show());          // expect: 40
}
