// What `sink` returns stores a `&'a i64` into `r`. If the type it returns
// were covariant in `'a`, `feed` could pass it a shorter-lived `&short`,
// and `r` would dangle. It's invariant, so `'a` must last as long as `r`
// is used.
fn sink<'b, 'a>(slot: &'b mut &'a i64) -> impl FnMut(&'a i64) + Discard { move |v| *slot = v }

fn feed<'s, F: FnMut(&'s i64) + Discard>(mut f: F, v: &'s i64) { f(v) }

fn main() {
    let long = 1;
    let mut r = &long;
    {
        let short = 2;
        feed(sink(&mut r), &short);
    }
    print(*r);
}
// error: `short` doesn't live long enough
