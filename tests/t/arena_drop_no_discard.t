// A value in an arena may point to something that dies before the arena,
// when nothing it holds has a `discard` of its own: dropping the arena
// then doesn't look at it.
use alloc::arena::Arena;

#[derive(Discard)]
struct Peek<'a> { r: &'a i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let a = Arena::new();
    let x = 5;
    let p = a.alloc(Peek { r: &x });
    out.line(p.r)?; // expect: 5
    Result::Ok(())
}
