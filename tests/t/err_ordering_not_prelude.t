// `Ordering` isn't in the prelude.
fn main() {
    let o = 1.cmp(&2);
    match o {
        Ordering::Less => print(1),
        _ => print(2),
    }
}
// error: unknown type `Ordering`
