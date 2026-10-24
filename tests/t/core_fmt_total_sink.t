// A sink that only counts is total, and so is formatting into it: `width`
// says no effects at all, not even `panic`, though it formats an `i64`.
use core::fmt::Write;
use core::num::wrapping_add;

#[derive(Discard)]
struct Count { n: usize }

impl Write for Count {
    type Error = !;
    effect e = {};

    fn write_str(&mut self, s: &str) -> Result<(), !> {
        self.n = wrapping_add(self.n, s.len());
        Result::Ok(())
    }

    fn write_char(&mut self, c: char) -> Result<(), !> {
        self.n = wrapping_add(self.n, c.len_utf8());
        Result::Ok(())
    }
}

/// How many bytes `x` takes in decimal.
fn width(x: i64) -> usize {
    let mut c = Count { n: 0 };
    c.put(&x);
    c.n
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&width(0))?;                    // expect: 1
    out.line(&width(-5))?;                   // expect: 2
    out.line(&width(123456))?;               // expect: 6
    out.line(&width(-9223372036854775808))?; // expect: 20
    Result::Ok(())
}
