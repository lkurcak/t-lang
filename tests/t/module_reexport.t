// `pub use` makes a name public somewhere else: the facade a library's
// prelude will be.
mod lib {
    mod inner {
        pub fn one() -> i64 { 1 }

        #[derive(Copy, Discard)]
        pub struct Unit { pub n: i64 }
    }

    pub use self::inner::{one, Unit as One};
}

use lib::One;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&lib::one())?;        // expect: 1
    let u = One { n: 5 };
    out.line(&u.n)?;               // expect: 5
    Result::Ok(())
}
