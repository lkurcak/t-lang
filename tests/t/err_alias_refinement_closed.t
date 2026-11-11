// An alias's predicate names only its binder, its const parameters and
// consts: nothing where it's used.
type Below = { v: u8 | v < n };

fn f(n: u8) -> u8 {
    let x: Below = 0;
    x + n
}

fn main() {
    let _ = f(3);
}
// error: cannot find value `n` in this refinement
