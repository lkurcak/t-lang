// Working out a projection picks the impl a type's sealed bounds allow:
// `<One<char> as Size>::Of` is `One<char>`'s impl's, not `One<T>`'s, as
// `char` can't be `Small` (see docs/29-closed-and-sealed-traits.md).
sealed trait Small {}

impl Small for u8 {}

#[derive(Copy, Discard)]
struct One<T> { x: T }

trait Size {
    type Of;

    fn of(&self) -> Self::Of;
}

impl<T: Small + Copy> Size for One<T> {
    type Of = T;

    fn of(&self) -> T { self.x }
}

impl Size for One<char> {
    type Of = bool;

    fn of(&self) -> bool { self.x == 'x' }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let b: bool = (One { x: 'x' }).of();
    let n: u8 = (One { x: 7u8 }).of();
    out.line(&b)?; // expect: true
    out.line(&n)?; // expect: 7
    Result::Ok(())
}
