// A test that's the first to use an array of `&str`s: compiled, the C
// struct of the array holds the struct of `&str`, which must come first
// (a bug `t test`'s compiled tests found in step 44).
fn main() {}

#[test]
fn words() {
    let w = ["a", "bc"];
    assert_eq(&w[1].len(), &2);
}
// test: words ... ok
