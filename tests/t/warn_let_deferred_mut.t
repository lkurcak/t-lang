// Given its only value by assignment, a variable needn't be `mut`.
fn main() {
    let mut r: i64;
    r = 1;
    print(r);       // expect: 1
}
// warning: variable `r` doesn't need to be mutable
