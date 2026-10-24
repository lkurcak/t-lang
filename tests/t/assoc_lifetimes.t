// What a projection stands for has the lifetimes of the type it's on, each
// in its place. Here `Out` is the second reference of a `Pair`, so the
// first may be short-lived.
#[derive(Discard)]
struct Pair<'a, 'b> { a: &'a i64, b: &'b i64 }

trait Second {
    type Out;

    fn second(self) -> Self::Out;
}

impl<'a, 'b> Second for Pair<'a, 'b> {
    type Out = &'b i64;

    fn second(self) -> &'b i64 { self.b }
}

fn get<T: Second>(t: T) -> T::Out { t.second() }

trait Tr {
    type Out;

    fn out(self) -> Self::Out;
}

impl<'a> Tr for &'a i64 {
    type Out = &'a i64;

    fn out(self) -> &'a i64 { self }
}

// A rigid projection lives as long as its type does.
fn need<'r, U: Discard + 'r>(u: U, r: &'r i64) -> i64 {
    let _u = u;
    *r
}

fn g<'r, T: Tr<Out: Discard> + 'r>(x: T::Out, r: &'r i64) -> i64 { need(x, r) }

// A reference to a projection: what it points to outlives it.
fn hold<'r, T: Tr>(x: &'r T::Out) -> &'r T::Out { x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut stdout = os.stdout;
    let long = 1;
    let r;
    {
        let short = 2;
        r = get(Pair { a: &short, b: &long });
        stdout.line(&(*Tr::out(&short)))?; // expect: 2
    }
    stdout.line(&(*r))?;                      // expect: 1
    stdout.line(&(g::<&i64>(&long, &long)))?; // expect: 1
    let x = &long;
    stdout.line(&(**hold::<&i64>(&x)))?; // expect: 1
    Result::Ok(())
}
