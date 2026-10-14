// Recursion that changes a type argument is fine, as long as it doesn't
// keep growing: `count::<bool>` needs `count::<int>`, which needs itself.
fn count<T>(_x: T, n: int) -> int {
    if n == 0 { 0 } else { 1 + count(n, n - 1) }
}
// So is swapping type parameters round.
fn flip<A, B>(a: A, b: B, n: int) -> int {
    if n == 0 { 0 } else { 1 + flip(b, a, n - 1) }
}
fn main() {
    print(count(true, 5));      // expect: 5
    print(flip(1, false, 4));   // expect: 4
}
