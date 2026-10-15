// A `fn` can be declared in any block. It means what it would at the top
// level; only its name is scoped, to the whole block it's in.
fn main() {
    // Used before it's declared: the name is in scope in all the block.
    print(double(1)); // expect: 2

    /// Doubles `x`.
    fn double(x: int) -> int { x * 2 }

    // Recursive.
    fn fact(n: int) -> int {
        if n == 0 { 1 } else { n * fact(n - 1) }
    }
    print(fact(5)); // expect: 120

    // Mutually recursive.
    fn is_even(n: int) -> bool {
        if n == 0 { true } else { is_odd(n - 1) }
    }
    fn is_odd(n: int) -> bool {
        if n == 0 { false } else { is_even(n - 1) }
    }
    print_bool(is_even(10)); // expect: true
    print_bool(is_odd(10));  // expect: false

    // Generic, and used at two types.
    fn twice<T>(f: fn(T) -> T, x: T) -> T { f(f(x)) }
    print(twice(double, 3));                 // expect: 12
    print_bool(twice::<bool>(|b| !b, true)); // expect: true

    // Nested in a nested `fn`, and in a closure's block.
    fn sum_to(n: int) -> int {
        fn go(i: int, acc: int, n: int) -> int {
            if i > n { acc } else { go(i + 1, acc + i, n) }
        }
        go(1, 0, n)
    }
    print(sum_to(4)); // expect: 10
    let k = 3;
    let add_k = |x: int| {
        fn inc(x: int) -> int { x + 1 }
        inc(x) + k
    };
    print(add_k(1)); // expect: 5

    // A function value like any other. It captures nothing, so unlike a
    // closure that captures, it can be returned, and stored.
    let f = pick(true);
    print(f(7));  // expect: 14
    print(top()); // expect: 1
}

fn pick(b: bool) -> fn(int) -> int {
    fn neg(x: int) -> int { -x }
    if b { double_it() } else { neg }
}

struct Holder { f: fn(int) -> int }

fn double_it() -> fn(int) -> int {
    fn double(x: int) -> int { x + x }
    let h = Holder { f: double };
    h.f
}

// Outer functions and types are fine to use.
struct One { n: int }
fn top() -> int {
    fn one() -> One { One { n: 1 } }
    one().n
}
