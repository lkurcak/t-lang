// Each closure that captures has a type of its own, which isn't a function
// type, so branches can't give one and a function.
fn pick(n: int, b: bool) -> int {
    let f = if b { |x: int| x + n } else { |x: int| x - 1 };
    f(1)
}
fn main() { print(pick(1, false)); }
// error: `if` and `else` have different types: `{closure}` and `fn(int) -> int`
