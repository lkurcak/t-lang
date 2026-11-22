// A field whose refinement ends `where` says nothing else of what's in
// it.
struct S<T> { t: T, n: { o: Option<{ v: u8 | v > 0 }> | o.is_some() where T: Copy } }

fn main() {}
// error: the refinement of `n` ends `where`, so it's all the field's type can say
