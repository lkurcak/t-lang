// A program that would mean something else if a bare uppercase name in a
// pattern could bind: `None` would match everything and `n` would be read
// from a `Some` it isn't. In T it's the prelude's `None`.
fn get(x: Option<i64>) -> i64 {
    match x {
        None => 0,
        Some(n) => n,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f"{get(Some(7))} {get(None)}")?; // expect: 7 0
    Ok(())
}

// A const parameter comes before an imported variant of the same name, as
// a name in a block does.
use Size::N;

#[derive(Discard)]
enum Size { N }

fn len<const N: usize>(_: [u8; N]) -> usize {
    N
}

#[test]
fn param_first() {
    assert_eq(&len([1, 2, 3]), &3);
    let Size::N = N;
}
// test: param_first ... ok
