fn one() -> int { 1 }

fn main() {
    one() = 2;
}
// error: can't assign to this: only to a variable, a field, or what `*` points to
