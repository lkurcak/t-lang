// A trait can be implemented for a function type, and a function value of
// that type has it: effect sets are the same when they have the same
// effects, in any order, and an impl's effect parameter stands for any
// set (docs/81).
use alloc::boxed::Box;

trait Twice {
    fn twice(&self, x: u32) -> u32;
}

impl Twice for fn(u32) -> u32 {
    fn twice(&self, x: u32) -> u32 { (*self)((*self)(x)) }
}

trait Name {
    fn name(&self) -> u32;
}

impl Name for fn(u32) -> u32 with {panic, alloc} {
    fn name(&self) -> u32 { 1 }
}

impl<effect e> Name for fn(u8) -> u8 with e {
    fn name(&self) -> u32 { 2 }
}

trait Out {
    type Out;
    fn out(&self) -> Self::Out;
}

impl Out for fn(u32) -> u32 {
    type Out = bool;
    fn out(&self) -> bool { (*self)(1) == 2 }
}

fn inc(x: u32) -> u32 with {} { x % 1000 + 1 }
fn loud(x: u32) -> u32 with {alloc, panic} {
    let b = Box::new(x);
    100 / *b
}
fn same(x: u8) -> u8 with {} { x }
fn halve(x: u8) -> u8 with panic { 200 / x }

fn names<T: Name>(t: &T) -> u32 { t.name() }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let f: fn(u32) -> u32 = inc;
    out.line(&f.twice(5))?; // expect: 7
    let g: fn(u32) -> u32 with {alloc, panic} = loud;
    out.line(&names(&g))?; // expect: 1
    let h: fn(u8) -> u8 = same;
    let k: fn(u8) -> u8 with panic = halve;
    out.line(&(h.name() + k.name()))?; // expect: 4
    out.line(&f.out())?; // expect: true
    Ok(())
}
