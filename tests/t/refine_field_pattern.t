// Taking a struct apart gives its fields' values, and what's known of
// them.
#[derive(Copy, Discard)]
struct Span {
    lo: { l: i64 | l >= 0 },
    hi: { h: i64 | h >= lo },
}

fn len(s: Span) -> i64 {
    let Span { lo, hi } = s;
    hi - lo
}

fn len_match(s: &Span) -> i64 {
    match s {
        Span { lo, hi } => *hi - *lo,
    }
}

fn main() {
    print(len(Span { lo: 1, hi: 4 })); // expect: 3
    print(len_match(&Span { lo: 2, hi: 4 })); // expect: 2
}
