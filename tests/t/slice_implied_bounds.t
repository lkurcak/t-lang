// A `&'a [&'b int]` is only a type if `'b` outlives `'a`, as for a `&'a
// &'b int`, so a function given one may count on it.
fn first<'a, 'b>(s: &'a [&'b int]) -> Option<&'a int> {
    match s.first() {
        Option::Some(r) => Option::Some(*r),
        Option::None => Option::None,
    }
}

fn main() {
    print_bool(first(core::slice::empty()).is_none()); // expect: true
}
