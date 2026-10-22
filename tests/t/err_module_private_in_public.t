mod m {
    #[derive(Discard)]
    struct Hidden { n: i64 }

    pub fn make() -> Hidden { Hidden { n: 1 } }
}

fn main() {}
// error: `make` is public, but its signature names `Hidden`, which is private to `m`
