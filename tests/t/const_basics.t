// A const item names a value fixed when the program is compiled: its
// initializer runs then, once, and each use of the name is that value.
const ANSWER: i64 = 42;
const DOUBLE: i64 = ANSWER * 2;
const YES: bool = DOUBLE > ANSWER;
const LETTER: char = 'x';
const GREETING: &str = "hello";
const BYTES: &[u8] = b"ab";
const NOTHING: () = ();

fn triple(n: i64) -> i64 { n * 3 }

// Calls run too, of whatever doesn't allocate, use `unsafe` or the world.
const NINE: i64 = triple(3);

mod limits {
    pub const MAX_USERS: usize = 1000;
}

// A use has no effects: a panic would have been found while compiling.
fn total() -> i64 with {} { ANSWER + 0 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&ANSWER)?;            // expect: 42
    out.line(&DOUBLE)?;            // expect: 84
    out.line(&YES)?;               // expect: true
    out.line(&LETTER)?;            // expect: x
    out.line(GREETING)?;           // expect: hello
    out.line(&BYTES.len())?;       // expect: 2
    out.line(&NINE)?;              // expect: 9
    out.line(&limits::MAX_USERS)?; // expect: 1000
    out.line(&total())?;           // expect: 42
    let () = NOTHING;
    Result::Ok(())
}
