// A const's name isn't a pattern: whether `N` there binds or compares
// would depend on what's in scope. Not yet: the name is kept for that.
const N: u8 = 3;

fn is_n(x: u8) -> bool {
    match x {
        N => true,
// warning: variable `N` should have a snake case name
// error: `N` is a const: a pattern can't name one yet
        _ => false,
    }
}

fn main() {
    let _ = is_n(3);
}
