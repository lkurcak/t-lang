// A derived `Ord` of a generic type needs its fields' `Ord`.
#[derive(Discard, Eq, Ord)]
struct W<T> { t: T }

#[derive(Discard, Eq)]
struct N {}

fn main() {
    let _less = W { t: N {} } < W { t: N {} };
}
// error: cannot compare `W<N>` with `<`: that needs `N: Ord`
