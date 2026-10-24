// Through a derived `Eq`, which compares field by field: `D<&V<T>>`'s
// compares a `V<&V<T>>`, which compares a `D<&V<&V<T>>>`, and so on.
#[derive(Discard, Eq)]
struct D<U> { v: V<U> }

#[derive(Discard)]
struct V<T> { x: T }

impl<T: Eq> Eq for V<T> {
    fn eq(&self, o: &V<T>) -> bool { false && D { v: V { x: self } } == D { v: V { x: o } } }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(V { x: 1i64 } == V { x: 1 }))?;
    Result::Ok(())
}
// error: polymorphic recursion: this use of `<D<U> as Eq>::eq` with `U` = `&V<T>` leads back
