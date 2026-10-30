// Packages can't depend on each other: each is checked against those it
// depends on, which must have been checked first.
// error: packages can't depend on each other: `a` depends on `b`, which depends on `a`
fn main() {
    let _x = a::f();
}
