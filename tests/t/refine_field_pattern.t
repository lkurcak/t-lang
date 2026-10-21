// Taking a struct apart gives its fields' values, and what's known of
// them.
#[derive(Copy, Discard)]
struct Span {
    lo: { l: int | l >= 0 },
    hi: { h: int | h >= lo },
}

fn len(s: Span) -> int {
    let Span { lo, hi } = s;
    hi - lo
}

fn len_match(s: &Span) -> int {
    match s {
        Span { lo, hi } => *hi - *lo,
    }
}

fn main() {
    print(len(Span { lo: 1, hi: 4 })); // expect: 3
    print(len_match(&Span { lo: 2, hi: 4 })); // expect: 2
}
