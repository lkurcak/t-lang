// `x.show()` only looks at traits in scope here: `Show` isn't.
mod show {
    pub trait Show {
        fn show(&self) -> int;
    }

    impl Show for super::P {
        fn show(&self) -> int { self.n }
    }
}

#[derive(Discard)]
struct P { n: int }

fn main() {
    let p = P { n: 1 };
    print(p.show());
}
// error: `P` has no method `show` in scope; trait `show::Show` has one: import it with `use show::Show;`
