// `core::array::from_fn(f)` makes each value from its index, so the values
// needn't be `Copy`; it does what `f` does.
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let squares: [u64; 4] = core::array::from_fn(|i| (i as u64) * (i as u64));
    out.line(&squares[3])?; // expect: 9
    let lists: [List<i64>; 2] = core::array::from_fn(|_| List::new());
    out.line(&lists[1].len())?; // expect: 0
    Result::Ok(())
}
