// Code for every integer type at once: `T: Int` says `T` is one of them,
// so its operators work, and so do literals that fit them all.
fn double<T: Int>(x: T) -> T with panic { x + x }

fn inc<T: Int>(x: T) -> Option<T> { checked_add(x, 1) }

// Below the greatest, so `+ 1` fits, whichever type it is.
fn succ<T: Int>(x: T) -> T {
    if x < core::num::greatest::<T>() { x + 1 } else { x }
}

fn sum<T: Int>(from: T, to: T) -> T with panic {
    let mut total = 0;
    for i in from..to {
        total = total + i;
    }
    total
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let b: u8 = 100;
    out.line(&double(b))?;           // expect: 200
    let s: i32 = -3;
    out.line(&double(s))?;           // expect: -6
    let full: u8 = 255;
    out.line(&inc(full).unwrap_or(0))?;      // expect: 0
    out.line(&inc(b).unwrap_or(0))?;         // expect: 101
    out.line(&succ(full))?;          // expect: 255
    out.line(&succ(s))?;             // expect: -2
    let lo: u16 = 1;
    out.line(&sum(lo, 5))?;          // expect: 10
    out.line(&(core::num::least::<i8>()))?;     // expect: -128
    out.line(&(core::num::greatest::<u32>()))?; // expect: 4294967295
    let none: i64 = 0;
    out.line(&sum(none, 3))?;        // expect: 3
    Result::Ok(())
}
