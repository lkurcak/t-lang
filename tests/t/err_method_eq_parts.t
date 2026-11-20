// An array whose values have no `Eq` has no `eq` either: the method is
// found by the built-in rule, which then asks it of the values.
#[derive(Discard)]
struct X {}

fn main() {
    let a = [X {}];
    let _ = a.eq(&a);
}
// error: `[X; 1]` doesn't implement `Eq`, which method `eq` is of; that needs `X: Eq`
