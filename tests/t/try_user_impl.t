// A program's own type can be `Try`: here, a parse that stops at its
// first error, with that error's position as what goes up.
use core::ops::{ControlFlow, Try};

#[derive(Discard)]
enum Parsed<T> { Done(T), Failed(usize) }

#[derive(Discard)]
struct At { at: usize }

impl<T> Try for Parsed<T> {
    type Output = T;
    type Residual = At;

    fn branch(self) -> ControlFlow<At, T> {
        match self {
            Parsed::Done(v) => ControlFlow::Continue(v),
            Parsed::Failed(at) => ControlFlow::Break(At { at }),
        }
    }

    fn from_residual(r: At) -> Parsed<T> { Parsed::Failed(r.at) }
}

fn digit(s: &str, at: usize) -> Parsed<u32> {
    match s.as_bytes().get(at) {
        Option::Some(b) => match *b {
            0x30..=0x39 => Parsed::Done(*b as u32 - 0x30),
            _ => Parsed::Failed(at),
        },
        Option::None => Parsed::Failed(at),
    }
}

fn two(s: &str) -> Parsed<u32> with panic {
    let a = digit(s, 0)?;
    let b = digit(s, 1)?;
    Parsed::Done(a * 10 + b)
}

fn show(out: &mut std::io::Stdout, p: Parsed<u32>) -> Result<(), std::io::Error> {
    match p {
        Parsed::Done(v) => out.line(&v),
        Parsed::Failed(at) => out.line(&at),
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    show(&mut out, two("42"))?; // expect: 42
    show(&mut out, two("4x"))?; // expect: 1
    show(&mut out, two("")) // expect: 0
}
