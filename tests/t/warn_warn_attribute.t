// `#[warn(..)]` turns lints on where `#[allow(..)]` turned them off: the
// innermost attribute that names a lint decides, and of two on the same
// thing, the last. `#![allow(..)]` at the top of a module is of all of it.
#![allow(unused_variables)]

fn quiet() {
    let a = 1i64;
}

#[warn(unused_variables)]
fn loud() {
    let b = 1i64;
    #[allow(unused_variables)]
    fn hushed() {
        let c = 1i64;
    }
    hushed();
}

#[warn(unused_variables)]
#[allow(unused_variables)]
fn last_wins() {
    let d = 1i64;
}

#[allow(unused_variables)]
#[warn(unused_variables, unused_variable)]
fn last_wins_again() {
    let e = 1i64;
}

mod m {
    #![warn(unused_variables)]

    pub fn f() {
        let g = 1i64;
    }
}

fn main() {
    quiet();
    loud();
    last_wins();
    last_wins_again();
    m::f();
}
// warning: unused variable `b`
// warning: unknown lint `unused_variable`
// warning: unused variable `e`
// warning: unused variable `g`
