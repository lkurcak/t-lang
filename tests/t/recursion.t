// Recursion and mutual recursion. Functions can be used before they're defined.

fn main() with panic {
    print(fact(10));        // expect: 3628800
    print(fib(20));         // expect: 6765
    print(b(is_even(10)));  // expect: 1
    print(b(is_odd(7)));    // expect: 1
    print(sum_to(5000));    // expect: 12502500
}

fn fact(n: int) -> int with panic { if n <= 1 { 1 } else { n * fact(n - 1) } }
fn fib(n: int) -> int with panic { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }
fn is_even(n: int) -> bool with panic { if n == 0 { true } else { is_odd(n - 1) } }
fn is_odd(n: int) -> bool with panic { if n == 0 { false } else { is_even(n - 1) } }
fn sum_to(n: int) -> int with panic { if n == 0 { 0 } else { n + sum_to(n - 1) } }
fn b(x: bool) -> int { if x { 1 } else { 0 } }
