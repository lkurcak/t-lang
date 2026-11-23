// An impl's own method for a generic type that no impl of the bound's
// trait can ever be for may have the effect: no call of it is one where
// the bound holds (lemma B6 of docs/92-where-a-bound-holds.md).
trait Small {
    fn size(&self) -> u8;
}

#[derive(Discard)]
struct A { n: u8 }

impl Small for A {
    fn size(&self) -> u8 { self.n }
}

#[derive(Discard)]
struct V<T> { n: u8, t: T }

trait Bump {
    fn bump(&self) -> u8 with panic unless Self: Small { 1 }
}

impl Bump for A {}

impl<T> Bump for V<T> {
    fn bump(&self) -> u8 with panic {
        if self.n > 5 {
            panic("too big");
        }
        self.n + 1
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let a = A { n: 9 };
    out.line(&a.bump())?; // expect: 1
    a.discard();
    let v = V { n: 3, t: 0u8 };
    out.line(&v.bump())?; // expect: 4
    v.discard();
    Ok(())
}
