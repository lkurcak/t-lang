// Two packages can't have one name in a program: `x::f` would be the
// symbol of two functions.
// error: two packages are called `x`: the one in `inner` and the one in `outer`
fn main() {
    let _a = x::f();
}
