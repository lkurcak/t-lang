// No warning where the `Err` case is needed: where the error type is a type
// parameter, or an associated type of one, it's `!` only in some uses; and
// `map_err` makes a `Result<T, !>` into one another `?` can pass on.
use core::fmt::Write;

fn get<E: Discard>(r: Result<i64, E>) -> i64 { r.unwrap_or(0) }

fn hi<W: Write>(w: &mut W) with {W::e, panic} where W::Error: Discard {
    w.write_str("hi").unwrap();
}

fn sure(x: i64) -> Result<i64, !> { Result::Ok(x) }

fn widen(x: i64) -> Result<i64, bool> with panic {
    let v = sure(x).map_err(|e| e)?;
    Result::Ok(v + 1)
}

#[derive(Discard)]
struct Out {}

impl Write for Out {
    type Error = !;
    effect e = {};

    fn write_str(&mut self, s: &str) -> Result<(), !> {
        print_str(s);
        Result::Ok(())
    }

    fn write_char(&mut self, _c: char) -> Result<(), !> { Result::Ok(()) }
}

fn main() with panic {
    print(get::<!>(sure(5)));       // expect: 5
    print(get(Result::Err(false))); // expect: 0
    hi(&mut Out {});                // expect: hi
    print(widen(1).unwrap_or(0));   // expect: 2
}
