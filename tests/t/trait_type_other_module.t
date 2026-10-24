// A trait can have the name of a type in another module: core's `Range`
// or alloc's `Iter`, here, or one of the program's own.
mod a {
    #[derive(Discard)]
    pub struct Foo { pub x: i64 }
}

trait Foo {
    fn f(&self) -> i64;
}

impl Foo for a::Foo {
    fn f(&self) -> i64 { self.x }
}

trait Range {}

trait Iter {}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(a::Foo { x: 3 }).f())?; // expect: 3
    Result::Ok(())
}
