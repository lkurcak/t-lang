// A string literal is a pattern: it matches a `&str` with the same bytes.
// `match` on words reads as a table, and alternatives, nesting, `let ..
// else` and match ergonomics work as for any pattern.
#[derive(Copy, Discard)]
enum Command { Help, Fill, Quit, Unknown }

fn command(word: &str) -> Command {
    match word {
        "help" | "h" | "?" => Command::Help,
        "fill" | "f" => Command::Fill,
        "quit" | "q" => Command::Quit,
        _ => Command::Unknown,
    }
}

fn code(c: Command) -> u8 {
    match c {
        Command::Help => 1,
        Command::Fill => 2,
        Command::Quit => 3,
        Command::Unknown => 0,
    }
}

// Nested in a variant, and the empty string.
fn first(w: Option<&str>) -> u8 {
    match w {
        Option::Some("") => 10,
        Option::Some("yes") => 11,
        Option::Some(_) => 12,
        Option::None => 13,
    }
}

// Through references: `&&str` is peeled.
fn count(words: &[&str]) -> u32 {
    let mut n: u32 = 0;
    for w in words.iter() {
        match w {
            "a" | "an" | "the" => {}
            _ => n = n + 1,
        }
    }
    n
}

// Not only literals: any `&str`, borrowed from anywhere, and characters
// beyond ASCII and escapes, byte for byte.
fn odd(s: &str) -> bool {
    let "é\n" = s else { return false };
    true
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&code(command("h")))?;
    out.line(&code(command("fill")))?;
    out.line(&code(command("q")))?;
    out.line(&code(command("hel")))?;
    out.line(&code(command("helpx")))?;
    out.line(&code(command("")))?;
    out.line(&first(Option::Some("")))?;
    out.line(&first(Option::Some("yes")))?;
    out.line(&first(Option::Some("ye")))?;
    out.line(&first(Option::None))?;
    let words = ["the", "cat", "an", "owl", "a"];
    out.line(&count(words.as_slice()))?;
    out.line(&odd("é\n"))?;
    out.line(&odd("e\n"))?;
    let line = "fill 3";
    let Option::Some(parts) = line.split_at(4) else { return Result::Ok(()) };
    out.line(&code(command(parts.head)))?;
    Result::Ok(())
}
// expect: 1
// expect: 2
// expect: 3
// expect: 0
// expect: 0
// expect: 0
// expect: 10
// expect: 11
// expect: 12
// expect: 13
// expect: 2
// expect: true
// expect: false
// expect: 2
