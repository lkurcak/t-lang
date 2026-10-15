// `_` still has to be inferred from something.
fn id<T>(x: T) -> T { x }
fn main() {
    let _f = id::<_>;
}
// error: cannot infer the type `_` stands for here; write it out instead
