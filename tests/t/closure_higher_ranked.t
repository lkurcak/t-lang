// A bound with references in it is for every lifetime, as a function type
// is.
fn app<F: Fn(&i64) -> &i64 + Discard>(f: F) -> i64 {
    let y = 1;
    *f(&y)
}

fn main() {
    let k: i64 = 2;
    print(app(|x| x));              // expect: 1
    print(app(|x| if k > 1 { x } else { x })); // expect: 1
}
