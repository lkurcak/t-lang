// Consts as lengths in generic code, and of arrays of values that must be
// dropped, made with `from_fn`: `[T; SIZE]` isn't `[T; 3]`, so it isn't
// made from a literal of three.
use alloc::boxed::Box;

const SIZE: usize = 3;

fn fill<T: Copy>(x: T) -> [T; SIZE] { [x; SIZE] }

fn zeros<const N: usize>() -> [u8; N] { [0; N] }

#[derive(Discard)]
struct Token { b: Box<usize> }

fn tokens() -> [Token; SIZE] with {alloc, panic} {
    core::array::from_fn(|i| Token { b: Box::new(i * 10) })
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    out.line(&fill(7u8)[2])?;         // expect: 7
    out.line(&zeros::<SIZE>().len())?; // expect: 3
    let t = tokens();
    out.line(&*t[2].b)?;              // expect: 20
    Ok(())
}
