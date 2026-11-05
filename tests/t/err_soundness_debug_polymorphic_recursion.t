// Through a derived `Debug`, which writes field by field: `D<&V<T>>`'s
// writes a `V<&V<T>>`, whose `fmt` writes a `D<&V<&V<T>>>`, and so on, a
// bigger type each time, so monomorphising would never end. The graph of
// what's copied has a node for each derived `fmt`, with its sink as a
// parameter, as it has one for each derived `Eq`'s `eq`.
use core::fmt::{Debug, Write};

#[derive(Debug, Discard)]
struct D<U> { v: V<U> }

#[derive(Discard)]
struct V<T> { x: T }

impl<T: Debug> Debug for V<T> {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        let d = D { v: V { x: self } };
        Debug::fmt(&d, w)
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let v = V { x: 1i64 };
    out.line(&f"{v:?}")?;
    Result::Ok(())
}
// error: polymorphic recursion: this use of `<D<U> as Debug>::fmt` with `U` = `&V<T>` leads back
