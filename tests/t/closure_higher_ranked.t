// A bound with references in it is for every lifetime, as a function type
// is.
fn app<F: Fn(&int) -> &int + Discard>(f: F) -> int {
    let y = 1;
    *f(&y)
}

fn main() {
    let k = 2;
    print(app(|x| x));              // expect: 1
    print(app(|x| if k > 1 { x } else { x })); // expect: 1
}
