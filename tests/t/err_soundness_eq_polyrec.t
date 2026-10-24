// `==` is a call of `eq`, so the check against polymorphic recursion sees
// it: this `eq` compares at `W<&W<T>>`, which compares at a bigger type
// again, and monomorphising it would never end.
#[derive(Discard)]
struct W<T> { x: T }

impl<T: Eq> Eq for W<T> {
    fn eq(&self, o: &W<T>) -> bool { false && W { x: self } == W { x: o } }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(W { x: 1i64 } == W { x: 1 }))?;
    Result::Ok(())
}
// error: polymorphic recursion: `<W<T> as Eq>::eq` uses itself with `T` = `&W<T>`
