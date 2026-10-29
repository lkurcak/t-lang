// An arena hands out `&mut`s from a `&`: many at once, each to a value
// of its own, which live as long as the borrow of the arena.
use alloc::arena::Arena;

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let a = Arena::new();
    let x = a.alloc(1i64);
    let y = a.alloc(2);
    *x = *x + 10;
    *y = *y + 20;
    out.line(&(*x + *y))?; // expect: 33
    out.line(&a.len())?;   // expect: 2
    Result::Ok(())
}
