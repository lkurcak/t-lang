// So that it can be equal to its value one day, two impls that would then
// overlap overlap already.
const N: usize = 3;

trait Name { fn name(&self) -> u8; }
impl Name for [u8; N] { fn name(&self) -> u8 { 1 } }
impl Name for [u8; 3] { fn name(&self) -> u8 { 2 } }
// error: overlap

fn main() {}
