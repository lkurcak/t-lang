// An atomic is changed through a shared reference, like a cell, but each
// change is one step no other thread can see half of: sequentially
// consistent, the only ordering there is.
use core::sync::Atomic;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a = Atomic::new(250u8);
    let r = &a;
    out.line(&r.fetch_add(10))?; // expect: 250
    out.line(&a.load())?; // expect: 4
    out.line(&a.fetch_sub(5))?; // expect: 4
    out.line(&a.load())?; // expect: 255
    a.store(12);
    out.line(&a.fetch_and(10))?; // expect: 12
    out.line(&a.fetch_or(1))?; // expect: 8
    out.line(&a.fetch_xor(3))?; // expect: 9
    out.line(&a.swap(7))?; // expect: 10
    match a.compare_exchange(7, 8) {
        Result::Ok(old) => out.line(&old)?, // expect: 7
        Result::Err(_) => {}
    }
    match a.compare_exchange(7, 9) {
        Result::Ok(_) => {}
        Result::Err(now) => out.line(&now)?, // expect: 8
    }
    let mut b = Atomic::new(false);
    out.line(&b.fetch_or(true))?; // expect: false
    out.line(&b.fetch_xor(true))?; // expect: true
    *b.get_mut() = true;
    out.line(&b.into_inner())?; // expect: true
    let n = Atomic::new(-1i64);
    n.store(n.load() - 1);
    out.line(&n.into_inner())?; // expect: -2
    Result::Ok(())
}
