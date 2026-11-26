// `p`'s type has three lifetimes: the message says which one it means.
#[derive(Copy, Discard)]
struct Tok<'s> { text: &'s str }

struct Parser<'a, 's> { toks: &'a [Tok<'s>], last: Tok<'s> }

fn expect<'a, 's>(p: &mut Parser<'a, 's>, tok: Tok<'s>) {
    p.last = tok;
}

fn exprs(p: &mut Parser, close: Tok) {
    expect(p, close);
}

fn main() {}
// error: the lifetime `'1` in the type of `p`
