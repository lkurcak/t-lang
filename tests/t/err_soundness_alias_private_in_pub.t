// A public alias of a private type would let other modules name what
// `m` keeps to itself.
mod m {
    #[derive(Discard)]
    struct Secret { n: i64 }
    pub type Open = Secret;
}

fn main() {}
// error: `Open` is public, but what it stands for names `Secret`, which is private to `m`
