// `assert_eq` says what each side is when it fails, by its `Debug`, so
// it needs one: `Display` isn't it.
#[derive(Discard, Eq)]
struct P { n: u8 }

impl core::fmt::Display for P {
    fn fmt<W: core::fmt::Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        w.write_str("P")
    }
}

fn main() {
    assert_eq(&P { n: 1 }, &P { n: 1 });
}
// error: `assert_eq` needs its `T` to implement `core::fmt::Debug`, and `P` doesn't; add `#[derive(Debug)]` to `P`
