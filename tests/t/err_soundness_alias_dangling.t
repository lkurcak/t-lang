// A lifetime through an alias is the lifetime written out.
type R<'a> = &'a i64;

fn dangle<'a>() -> R<'a> {
    let x = 1i64;
    &x
}

fn main() {}
// error: `x` doesn't live long enough: it goes away when the function returns, but it's borrowed for longer than that
