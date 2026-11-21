// A function saying `with panic if P` may panic only where `P` held when it
// was called: each site that may panic in its body is checked to be where
// `P` holds, or to panic only if it does (docs/90-conditional-effects.md).

// An operator that overflows only when the condition holds.
fn add(a: u8, b: u8) -> u8 with panic if a > 255 - b { a + b }

// A call whose own condition follows from the caller's.
fn add_small(a: u8, b: u8) -> u8 with panic if a > 200 {
    if b > 55 {
        return 0;
    }
    add(a, b)
}

// A panic in a branch only taken when the condition holds, of a
// parameter as it was given, whatever it is later.
fn halve(mut x: u8) -> u8 with panic if x % 2 == 1 {
    if x % 2 == 1 {
        panic("odd");
    }
    x = x / 2;
    x
}

// Of what's behind a `&mut`, as it was at the call.
fn take_last(l: &mut alloc::list::List<i64>) -> i64 with panic if l.len() == 0 {
    match l.pop() {
        Some(x) => x,
        None => panic("empty"),
    }
}

// More than one effect, each with its own condition, or none.
fn both(l: &mut alloc::list::List<i64>, x: i64) -> i64
with {alloc if l.len() == l.capacity(), panic if x == 0}
{
    l.push(x);
    100 / x
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    out.line(&add(100, 155))?;      // expect: 255
    out.line(&add_small(200, 50))?; // expect: 250
    out.line(&halve(8))?;           // expect: 4
    let mut l = alloc::list::List::new();
    l.push(9i64);
    out.line(&take_last(&mut l))?; // expect: 9
    out.line(&both(&mut l, 5))?;   // expect: 20
    l.discard();
    Ok(())
}
