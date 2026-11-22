// A field's refinement hangs on a trait seen at least as far as the field
// is: whoever can write the field must keep the refinement where the
// bound holds, so must be able to see the bound.
mod m {
    trait Small {
        #[measure]
        fn size(&self) -> u8;
    }

    pub struct Kept<T> { pub of: T, pub n: { v: u8 | v <= of.size() where T: Small } }
}

fn main() {}
// error: field `n` is public, but its type names `Small`, which is private to `m`
