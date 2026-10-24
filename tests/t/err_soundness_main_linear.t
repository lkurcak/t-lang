// The error `main` gives back is written out, and then thrown away at the
// end, as nothing can use it up. So it must be `Discard`, or a file
// descriptor, or a buffer, would leak.
struct Fd { n: i64 }

impl core::fmt::Display for Fd {
    fn fmt<W: core::fmt::Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        w.put(&self.n)
    }
}

fn main() -> Result<(), Fd> { Result::Err(Fd { n: 1 }) }
// error: the error `main` gives back is thrown away once it's written out, so it must be `Discard`, and a `Fd` isn't
