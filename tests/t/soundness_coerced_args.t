// A coerced argument is a borrow of a place, `&**r`, taken after the other
// arguments like any other: it sees what they wrote there.

use alloc::boxed::Box;

fn first(a: &i64, _n: i64) -> i64 { *a }

fn get(r: &i64) -> i64 { *r }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut one = Box::new(1);
    let rb = &mut one;
    out.line(&first(rb, { **rb = 2; 0 }))?; // expect: 2
    let mut b = Box::new(3);
    out.line(&get(&mut b))?;                 // expect: 3
    b = Box::new(4);
    out.line(&(*b))?;                          // expect: 4
    Ok(())
}
