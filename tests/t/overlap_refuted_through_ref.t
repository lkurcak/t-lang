// Two impls whose types could be the same don't overlap if, where they
// would, one needs what no impl could give there: `&Bag: Show` would need
// `Bag: Show`, and `Bag` and `Show` are both this program's, so every impl
// that could say so is here, and none does.
trait Show {}

trait Tr {
    fn which(&self) -> i64;
}

impl<T: Show> Tr for T {
    fn which(&self) -> i64 { 1 }
}

impl<'a, T: Show> Show for &'a T {}

impl Show for i64 {}

#[derive(Discard)]
struct Bag {}

impl<'a> Tr for &'a Bag {
    fn which(&self) -> i64 { 2 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let b = Bag {};
    let n = 5i64;
    out.line(&Tr::which(&&b))?; // expect: 2
    out.line(&Tr::which(&&n))?; // expect: 1
    out.line(&n.which()) // expect: 1
}
