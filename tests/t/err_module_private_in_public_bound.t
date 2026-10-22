mod m {
    trait Secret {
        fn s(&self) -> i64;
    }

    pub fn use_it<T: Secret>(x: &T) -> i64 { x.s() }
}

fn main() {}
// error: `use_it` is public, but its signature names `Secret`, which is private to `m`
