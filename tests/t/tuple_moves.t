// A tuple owns its values: moving one out leaves the rest, a new value
// can go back in, and dropping the tuple drops what's left, field by
// field. A tuple holding a value that isn't `Discard` isn't either, so
// it must be taken apart: the interpreter fails on a box that's never
// freed, or freed twice.
use alloc::boxed::Box;
use alloc::string::String;

struct Token { id: u8 }

fn spend(t: Token) -> u8 {
    let Token { id } = t;
    id
}

fn pair() -> (Token, Box<i64>) { (Token { id: 4 }, Box::new(9)) }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut t = (String::from("ab"), Box::new(3i64));
    let s = t.0;
    out.line(&s.len())?;          // expect: 2
    t.0 = String::from("xyz");
    out.line(&t.0.len())?;        // expect: 3
    out.line(&(*t.1))?;           // expect: 3
    // The linear `Token` is moved out and used up; the box is dropped.
    let (tok, b) = pair();
    out.line(&spend(tok))?;       // expect: 4
    out.line(&(*b))?;             // expect: 9
    let p = pair();
    out.line(&spend(p.0))?;       // expect: 4
    let nested = ((Box::new(1i64), String::from("q")), Box::new(2i64));
    let ((x, _), _) = nested;
    out.line(&(*x))?;             // expect: 1
    Ok(())
}
