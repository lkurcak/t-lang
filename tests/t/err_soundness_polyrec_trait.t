// Polymorphic recursion through impls of a trait: `W<T>`'s `go` uses `go`
// at `W<&W<T>>`.
trait Tr {
    fn go(&self, n: i64) -> i64 with panic;
}

#[derive(Discard)]
struct W<T> { x: T }

impl Tr for i64 {
    fn go(&self, n: i64) -> i64 with panic {
        if n == 0 { *self } else { W { x: *self }.go(n - 1) }
    }
}

impl<T: Tr> Tr for W<T> {
    fn go(&self, n: i64) -> i64 with panic {
        if n == 0 { 0 } else { W { x: self }.go(n - 1) }
    }
}

impl<'a, T: Tr> Tr for &'a T {
    fn go(&self, n: i64) -> i64 with panic { (*self).go(n) }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&1i64.go(3))?;
    Result::Ok(())
}
// error: polymorphic recursion: `<W<T> as Tr>::go` uses itself with `T` = `&W<T>`
