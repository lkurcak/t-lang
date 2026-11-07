// Through a `&mut`, what's in an `Unsafe` is lent safely: nothing else can
// reach it while the loan lives.
use core::cell::Unsafe;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut c = Unsafe::new(3u32);
    *c.get_mut() = 4;
    let mut out = os.stdout;
    out.line(&c.into_inner())?; // expect: 4
    Ok(())
}
