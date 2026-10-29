type R<'a> = &'a i64;

fn f<'a, 'b>(r: R<'a, 'b>) {}

fn main() {}
// error: `R` takes 1 lifetime argument but 2 were supplied
