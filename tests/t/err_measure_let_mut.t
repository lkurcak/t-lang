// A `mut` variable's refinement is checked at each assignment, while what's
// behind a `&mut` can change in between.
fn f(s: &mut alloc::list::List<i64>) with alloc {
    let mut i: { i: usize | i <= s.len() } = 0;
    s.push(1);
    i = 1;
}

fn main() {}
// error: the refinement of a `mut` variable can't name a measure of what's behind `&mut` `s`, which can change
