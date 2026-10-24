// A derived impl needs exactly what makes its fields implement the trait,
// which can be another trait: `Q<T>: Eq` needs `T: Show`, so `W<T>`'s
// derived `Eq` does too. It used to need `T: Eq` instead, so `W<i64>`
// passed, and monomorphisation then couldn't find `i64: Show`.
trait Show {
    fn show(&self) -> i64;
}

#[derive(Discard)]
struct Q<T> { x: T }

impl<T: Show> Eq for Q<T> {
    fn eq(&self, o: &Q<T>) -> bool { self.x.show() == o.x.show() }
}

#[derive(Discard, Eq)]
struct W<T> { q: Q<T> }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a = W { q: Q { x: 1i64 } };
    let b = W { q: Q { x: 1 } };
    out.line(&(a == b))?;
    Result::Ok(())
}
// error: cannot compare `W<i64>` with `==`: that needs `i64: Show`
