// Matching ints needs every int covered, by a catch-all or by ranges. The
// counterexample is the first range of ints left out.
fn f(n: int) -> int {
    match n { 0 => 1, 1 => 2 }
}
fn main() {}
// error: non-exhaustive `match`: `..=-1` not covered
