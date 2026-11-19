// A blanket impl forwards what its trait says, which the impl it calls
// gives it (docs/86-trait-contracts.md).
trait Small {
    fn get(&self) -> { v: usize | v < 10 };
}

#[derive(Discard)]
struct S {}

impl Small for S {
    fn get(&self) -> usize { 9 }
}

impl<'a, T: Small> Small for &'a T {
    fn get(&self) -> usize { (**self).get() }
}

fn index<T: Small>(t: &T) -> u8 {
    let a = [5u8; 10];
    a[t.get()]
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let s = S {};
    out.line(&index(&&s))?; // expect: 5
    Ok(())
}
