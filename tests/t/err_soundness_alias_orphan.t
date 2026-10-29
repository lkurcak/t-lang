// An impl for an alias is an impl for what it stands for, under the same
// rules: `Option<L>`'s inherent impls are `core::option`'s.
struct L {}
type O = Option<L>;

impl O {
    fn get(&self) -> i64 { 1 }
}

fn main() {}
// error: an inherent impl of `Option<L>` must be in its module, `core::option`
