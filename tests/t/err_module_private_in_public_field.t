mod m {
    #[derive(Discard)]
    struct Hidden { n: int }

    #[derive(Discard)]
    pub struct Open { pub h: Hidden }
}

fn main() {}
// error: field `h` is public, but its type names `Hidden`, which is private to `m`
