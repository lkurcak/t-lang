export trait Go {
    effect e;

    fn go(&self) -> u8 with Self::e;
}

#[derive(Discard where T: Discard)]
export struct W<T> { x: T }

impl<T: Discard> Go for W<T> {
    effect e = panic unless T: Ord;

    fn go(&self) -> u8 with Self::e { 1 }
}

#[derive(Discard, Eq, Ord)]
export struct C { n: u8 }

#[derive(Discard where T: Discard)]
export struct Boom<T> { x: T }

impl Go for Boom<W<C>> {
    effect e = <W<C> as Go>::e;

    fn go(&self) -> u8 with Self::e { panic("boom") } // error: `panic` may panic, but `go` doesn't say `with panic`
}

export fn boom() -> Boom<W<C>> { Boom { x: W { x: C { n: 1 } } } }
