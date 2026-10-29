// A projection in an alias names its trait: it's worked out where the
// alias is used, for the type it's used with.
type ItemOf<I> = <I as Iterator>::Item;

fn first<I: Iterator + Discard>(mut it: I) -> Option<ItemOf<I>> with I::e {
    it.next()
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let r = core::ops::Range::new(4i64, 9);
    let n: ItemOf<core::ops::Range<i64>> = match first(r) {
        Option::Some(n) => n,
        Option::None => 0,
    };
    out.line(&n)?; // expect: 4
    Result::Ok(())
}
