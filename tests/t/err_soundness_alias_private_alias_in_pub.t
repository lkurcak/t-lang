// A private alias is a private item: a public signature can't name it,
// even if what it stands for is public.
mod m {
    type Num = i64;
    pub fn get() -> Num { 1 }
}

fn main() { let _x = m::get(); }
// error: `get` is public, but its signature names `Num`, which is private to `m`
