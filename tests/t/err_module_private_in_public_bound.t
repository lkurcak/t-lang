mod m {
    trait Secret {
        fn s(&self) -> int;
    }

    pub fn use_it<T: Secret>(x: &T) -> int { x.s() }
}

fn main() {}
// error: `use_it` is public, but its signature names `Secret`, which is private to `m`
