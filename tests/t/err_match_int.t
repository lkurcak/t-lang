// There are too many ints to list, so matching on them needs a catch-all.
fn f(n: int) -> int {
    match n { 0 => 1, 1 => 2 }
}
fn main() {}
// error: non-exhaustive `match`: `_` not covered
