// Bounds hold of what an alias stands for: an impl for it is an impl for
// that, and a bound on it is checked of that.
use core::fmt::{Display, Write};

struct Celsius { t: i64 }
type Temp = Celsius;

impl Display for Temp {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        self.t.fmt(w)?;
        w.write_str("C")
    }
}

type Shown<T> = T;

fn show<T: Display>(x: &Shown<T>, out: &mut std::io::Stdout) -> Result<(), std::io::Error> {
    out.line(x)
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let c: Temp = Celsius { t: 21 };
    show(&c, &mut out)?; // expect: 21C
    Result::Ok(())
}
