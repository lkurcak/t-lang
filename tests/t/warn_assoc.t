// Lints know associated types: their names are types' names, in
// `UpperCamelCase`, and a variable of a projection's type is a variable
// like any other.
trait Tr {
    type item: Discard;

    fn out(&self) -> Self::item;
}

#[derive(Discard)]
struct A { n: i64 }

impl Tr for A {
    type item = i64;

    fn out(&self) -> i64 { self.n }
}

fn f<T: Tr>(t: &T) -> i64 {
    let x = t.out();
    1
}

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&f(&A { n: 1 }))?;  Result::Ok(()) } // expect: 1
// warning: associated type `item` should have an upper camel case name
// warning: unused variable `x`
