type Pair<T> = Option<T>;

fn f(p: Pair) {}

fn main() {}
// error: `Pair` takes 1 type argument but 0 were supplied
