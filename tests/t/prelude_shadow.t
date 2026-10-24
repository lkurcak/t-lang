// A program's own items come before the prelude's: this `Option` is not
// `core`'s, which is still there by its path.

#[derive(Discard)]
enum Option { Some(i64), None }

fn mine(n: i64) -> Option { if n > 0 { Option::Some(n) } else { Option::None } }

fn theirs(n: i64) -> core::option::Option<i64> { core::option::Option::Some(n) }

mod inner {
    // A module sees the prelude, not the crate root's `Option`.
    pub fn wrap(n: i64) -> Option<i64> { Option::Some(n) }
}

// Nor can a program's own `Iterator`, which isn't what `for` uses.
trait Iterator {
    fn next(&mut self) -> i64;
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut sum: i64 = 0;
    for i in 0..4 {
        sum = sum + i;
    }
    out.line(&sum)?; // expect: 6
    match mine(1) {
        Option::Some(n) => out.line(&n)?, // expect: 1
        Option::None => {}
    }
    out.line(&theirs(2).unwrap_or(0))?;      // expect: 2
    out.line(&inner::wrap(3).unwrap_or(0))?; // expect: 3
    Result::Ok(())
}
