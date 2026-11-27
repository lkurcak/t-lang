// Where a loop's `break` and its end meet, a refined `let`'s value is
// taken to have its refinement (lemma J3 of docs/61-proof-search.md),
// which holds as every write proves it: one through a `&mut` too, so
// this one, which would break it, is refused.
fn need(g: { g: u32 | g <= 5 }) -> u32 with {} { g }

fn spoil(x: u32) -> u32 with {} {
    let mut g: { g: u32 | g <= 5 } = 5;
    for _ in 0..5u32 {
        if x == 3 {
            break;
        }
        let r = &mut g;
        *r = 100;
    }
    need(g)
}
// error: can't borrow `g` as `&mut`: its type has a refinement, which a write through the reference could break

fn main() {}
