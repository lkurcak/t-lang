// What an alias stands for is checked where it's used: `S<N>` is
// `Sorted<N>`, which needs `N: Ord`.
struct Sorted<T: Ord> { t: T }
type S<T> = Sorted<T>;
struct N {}

fn f(_s: S<N>) {}

fn main() {}
// error: `Sorted` needs its `T` to implement `Ord`, and `N` doesn't
