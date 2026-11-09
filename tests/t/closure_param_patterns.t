// A closure's parameter can be a pattern that always matches: a tuple's,
// a struct's, `_`. Through a reference, it binds references, as a `let`
// does.
use alloc::list::List;

#[derive(Copy, Discard)]
struct P { x: i64, y: i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let add = |(a, b): (i64, i64)| a + b;
    out.line(&add((2, 3)))?;             // expect: 5
    let norm = |P { x, y }| x * x + y * y;
    out.line(&norm(P { x: 3, y: 4 }))?;  // expect: 25
    let mut l: List<(u8, char)> = List::new();
    l.push((1, 'a'));
    l.push((2, 'b'));
    l.push((3, 'c'));
    let big = l.iter().filter(|(n, _)| *n >= 2).count();
    out.line(&big)?;                     // expect: 2
    let total = l.iter().map(|(n, _)| *n as u32).fold(0, |acc, n| acc + n);
    out.line(&total)?;                   // expect: 6
    let ignore = |_, y: u8| y;
    out.line(&ignore('z', 9))?;          // expect: 9
    Ok(())
}
