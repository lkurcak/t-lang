// Through a blanket impl.
#[derive(Discard)]
struct W<T> { x: T }

trait Tr {
    fn deep(self, n: i64) -> i64 with panic;
}

impl<T: Discard> Tr for T {
    fn deep(self, n: i64) -> i64 with panic {
        if n == 0 { 0 } else { W { x: self }.deep(n - 1) }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&1i64.deep(3))?;
    Result::Ok(())
}
// error: polymorphic recursion: `<T as Tr>::deep` uses itself with `T` = `W<T>`
