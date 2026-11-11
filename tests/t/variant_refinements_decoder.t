// Part way through a character in UTF-8: how many bytes are still to come,
// and the number so far, whose bound depends on how many: so the shift
// can't overflow, and the next state is proved to keep it. (t-libs'
// `term` masked the number before shifting, as this couldn't be said.)
#[derive(Copy, Discard)]
enum State {
    Start,
    Utf8(
        { left: u8 | left >= 1 && left <= 3 },
        { c: u32 | (left == 1 && c < 0x8000) || (left == 2 && c < 0x200) || (left == 3 && c < 8) },
    ),
}

fn start(b: u8) -> State {
    match b {
        0xC2..=0xDF => State::Utf8(1, (b & 0x1F) as u32),
        0xE0..=0xEF => State::Utf8(2, (b & 0x0F) as u32),
        0xF0..=0xF4 => State::Utf8(3, (b & 0x07) as u32),
        _ => State::Start,
    }
}

fn step(s: State, b: u8) -> (State, Option<u32>) {
    match s {
        State::Start => (start(b), None),
        State::Utf8(left, code) => {
            let code = code * 64 + (b & 0x3F) as u32;
            match left > 1 {
                true => (State::Utf8(left - 1, code), None),
                false => (State::Start, Some(code)),
            }
        },
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    // `€`, U+20AC: E2 82 AC.
    let (s, _) = step(State::Start, 0xE2);
    let (s, _) = step(s, 0x82);
    let (_, c) = step(s, 0xAC);
    out.line(&c.unwrap_or(0))?; // expect: 8364
    Ok(())
}
