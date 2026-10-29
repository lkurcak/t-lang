// What an arena hands out lives no longer than the arena.
use alloc::arena::Arena;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let r;
    {
        let a = Arena::new();
        r = a.alloc(5i64);
    }
    out.line(r)?;
    Result::Ok(())
}
// error: `a` doesn't live long enough
