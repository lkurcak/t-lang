// `pub use` makes a name public somewhere else: the facade a library's
// prelude will be.
mod lib {
    mod inner {
        pub fn one() -> int { 1 }

        #[derive(Copy, Discard)]
        pub struct Unit { pub n: int }
    }

    pub use self::inner::{one, Unit as One};
}

use lib::One;

fn main() {
    print(lib::one());        // expect: 1
    let u = One { n: 5 };
    print(u.n);               // expect: 5
}
