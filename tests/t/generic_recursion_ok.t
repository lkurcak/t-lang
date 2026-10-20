// Recursion that changes a type argument is fine, as long as it doesn't
// keep growing: `count::<bool>` needs `count::<int>`, which needs itself.
fn count<T: Discard>(_x: T, n: int) -> int with panic {
    if n == 0 { 0 } else { 1 + count(n, n - 1) }
}
// So is swapping type parameters round.
fn flip<A: Discard, B: Discard>(a: A, b: B, n: int) -> int with panic {
    if n == 0 { 0 } else { 1 + flip(b, a, n - 1) }
}
fn main() with panic {
    print(count(true, 5));      // expect: 5
    print(flip(1, false, 4));   // expect: 4
}
