// Each call wraps `g` in a new closure, whose type holds `G`: the copies
// monomorphisation would make never end.
fn f<G: Fn() -> int + Discard>(g: G, n: int) -> int {
    if n == 0 { g() } else { f(|| g() + 1, n - 1) }
}

fn main() {
    print(f(|| 0, 3));
}
// error: polymorphic recursion
