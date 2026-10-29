// An arena can't move while what it handed out lives: its values would
// still be freed with it, wherever it went.
use alloc::arena::Arena;

fn take(_a: Arena<i64>) {}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let a = Arena::new();
    let r = a.alloc(5i64);
    take(a);
    out.line(r)?;
    Result::Ok(())
}
// error: cannot move out of `a` while `a` is borrowed
