// A function type in an impl's type says its effects exactly, or as one
// effect parameter: `{panic, e}` could be `{panic}` with `e` either `{}` or
// `{panic}`, so which `e` an impl is at wouldn't be a function of the type.
trait Name {
    fn name(&self) -> u32;
}

impl<effect e> Name for fn(u32) -> u32 with {panic, e} {
    fn name(&self) -> u32 { 1 }
}

fn main() {}
// error: a set of effects in an impl's type is written out in full, or is one effect parameter alone, not `{panic, e}`: so that matching a type to it says what each parameter is (see docs/81-contradictions-and-fn-types.md)
