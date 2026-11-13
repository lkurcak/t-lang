// A type with a parameter in it, `Wrap<T>`, matches only the blanket
// impl's type, but `Wrap<Foo>` matches another's too, apart from it as
// `Wrap<Foo>` can't be `B`: so what the blanket impl says of `Wrap<T>` is
// said only where its bound holds, and here it isn't known to (hole W1,
// found while making step 73).
trait B {}

trait Tr {
    type A;

    fn get(&self) -> Self::A;
}

impl<X: B> Tr for X {
    type A = i64;

    fn get(&self) -> i64 { 5 }
}

#[derive(Discard)]
struct Wrap<T> { t: T }

#[derive(Discard)]
struct Foo {}

impl Tr for Wrap<Foo> {
    type A = bool;

    fn get(&self) -> bool { true }
}

trait Get {
    fn g(&self) -> i64 where Self: Tr;
}

impl<T> Get for Wrap<T> {
    fn g(&self) -> i64 where Self: Tr { self.get() } // error: expected `i64`, found `<Wrap<T> as Tr>::A`
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n = (Wrap { t: Foo {} }).g();
    out.line(&(n + 1))
}
