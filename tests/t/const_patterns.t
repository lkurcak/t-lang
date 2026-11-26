// A const in a pattern matches the value it names: a path, as any name
// that starts with an uppercase letter is in a pattern (step 45), so it
// never binds. Of an integer type, `bool`, `char` or `&str`, compared as
// a literal of its value would be.
const CMP_PREC: u8 = 3;
const LAST: usize = 3;
const STOP: char = '.';
const ON: bool = true;
const KEYWORD: &str = "fn";

mod limits {
    pub const TOP: i32 = 100;
}

fn prec(next: Option<(u8, u8)>) -> u8 {
    if let Some((_, CMP_PREC)) = next { 1 } else { 0 }
}

fn word(s: &str) -> u8 {
    match s {
        KEYWORD => 1,
        "let" => 2,
        _ => 0,
    }
}

fn top(n: i32) -> bool {
    match n {
        limits::TOP => true,
        _ => false,
    }
}

// The arm knows the value is the const's: `i == 3`, so `a[i]` is in
// bounds.
fn at_last(a: [u8; 4], i: usize) -> u8 {
    match i {
        LAST => a[i],
        _ => 0,
    }
}

// A const parameter too.
fn is_len<const N: usize>(a: [u8; N], n: usize) -> bool {
    let _ = a;
    match n {
        N => true,
        _ => false,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&prec(Some((0, 3))))?; // expect: 1
    out.line(&prec(Some((0, 4))))?; // expect: 0
    out.line(&word("fn"))?; // expect: 1
    out.line(&word("let"))?; // expect: 2
    out.line(&top(100))?; // expect: true
    out.line(&top(99))?; // expect: false
    out.line(&at_last([1, 2, 3, 4], 3))?; // expect: 4
    out.line(&is_len([0, 0], 2))?; // expect: true
    out.line(&is_len([0, 0], 3))?; // expect: false
    let c = '.';
    match &c {
        STOP => out.line(&"stop")?, // expect: stop
        _ => out.line(&"go")?,
    }
    match ON {
        ON => out.line(&"on")?, // expect: on
        _ => out.line(&"off")?,
    }
    Ok(())
}
