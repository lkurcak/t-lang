// An impl's effect parameter stands for any set: so this impl is for
// `fn(u32) -> u32 with panic` too, and the two overlap.
trait Name {
    fn name(&self) -> u32;
}

impl<effect e> Name for fn(u32) -> u32 with e {
    fn name(&self) -> u32 { 1 }
}

impl Name for fn(u32) -> u32 with panic {
    fn name(&self) -> u32 { 2 }
}

fn main() {}
// error: this impl of `Name` overlaps `impl Name for fn(u32) -> u32 with e`: a type can be both `fn(u32) -> u32 with panic` and `fn(u32) -> u32 with e`, and then which impl it uses would be a choice
