// A `fn` can be declared in any block. It means what it would at the top
// level; only its name is scoped, to the whole block it's in.
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    // Used before it's declared: the name is in scope in all the block.
    out.line(&double(1))?; // expect: 2

    /// Doubles `x`.
    fn double(x: i64) -> i64 with panic { x * 2 }

    // Recursive.
    fn fact(n: i64) -> i64 with panic {
        if n == 0 { 1 } else { n * fact(n - 1) }
    }
    out.line(&fact(5))?; // expect: 120

    // Mutually recursive.
    fn is_even(n: i64) -> bool with panic {
        if n == 0 { true } else { is_odd(n - 1) }
    }
    fn is_odd(n: i64) -> bool with panic {
        if n == 0 { false } else { is_even(n - 1) }
    }
    out.line(&is_even(10))?; // expect: true
    out.line(&is_odd(10))?;  // expect: false

    // Generic, and used at two types.
    fn twice<T>(f: fn(T) -> T with panic, x: T) -> T with panic { f(f(x)) }
    out.line(&twice(double, 3))?;                 // expect: 12
    out.line(&(twice::<bool>(|b| !b, true)))?; // expect: true

    // Nested in a nested `fn`, and in a closure's block.
    fn sum_to(n: i64) -> i64 with panic {
        fn go(i: i64, acc: i64, n: i64) -> i64 with panic {
            if i > n { acc } else { go(i + 1, acc + i, n) }
        }
        go(1, 0, n)
    }
    out.line(&sum_to(4))?; // expect: 10
    let k = 3;
    let add_k = |x: i64| {
        fn inc(x: i64) -> i64 with panic { x + 1 }
        inc(x) + k
    };
    out.line(&add_k(1))?; // expect: 5

    // A function value like any other. It captures nothing, so unlike a
    // closure that captures, it can be returned, and stored.
    let f = pick(true);
    out.line(&f(7))?;  // expect: 14
    out.line(&top())?; // expect: 1
    Result::Ok(())
}

fn pick(b: bool) -> fn(i64) -> i64 with panic {
    fn neg(x: i64) -> i64 with panic { -x }
    if b { double_it() } else { neg }
}

#[derive(Discard)]
struct Holder { f: fn(i64) -> i64 with panic }

fn double_it() -> fn(i64) -> i64 with panic {
    fn double(x: i64) -> i64 with panic { x + x }
    let h = Holder { f: double };
    h.f
}

// Outer functions and types are fine to use.
#[derive(Discard)]
struct One { n: i64 }
fn top() -> i64 {
    fn one() -> One { One { n: 1 } }
    one().n
}
