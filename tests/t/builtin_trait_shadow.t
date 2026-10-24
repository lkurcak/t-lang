// The traits the language is built on are core's items, in the prelude:
// a program can name its own traits after them, and still reach core's by
// their paths.
trait Copy {
    fn copy(&self) -> i64;
}

#[derive(Discard)]
struct S { n: i64 }

impl Copy for S {
    fn copy(&self) -> i64 { self.n }
}

// Copying out from behind a reference needs core's `Copy`.
fn get<T: core::mem::Copy>(x: &T) -> T { *x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let s = S { n: 7 };
    out.line(&s.copy())?; // expect: 7
    out.line(&get(&3i64))?; // expect: 3
    Result::Ok(())
}
