// Only integers and `bool`s are in the logic.
const NAME: &str = "t";

fn named(n: { n: usize | n == NAME }) -> usize { n }
// error: a refinement can't name `NAME`, a const of type `&str`

fn main() {
    let _ = named(1);
}
