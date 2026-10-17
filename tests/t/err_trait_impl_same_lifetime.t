// Resolution would find this impl for a `Pair<'x, 'y>` too, and `leak`,
// checked knowing the lifetimes are the same, would hand back `&inner` as
// if it lived as long as `outer`.
struct Pair<'a, 'b> { long: &'a int, short: &'b int }

trait Leak { fn leak(self) -> Self; }

impl<'a> Leak for Pair<'a, 'a> {
    fn leak(self) -> Pair<'a, 'a> { Pair { long: self.short, short: self.short } }
}

fn main() {
    let outer = 1;
    let l;
    {
        let inner = 2;
        let p = Pair { long: &outer, short: &inner };
        l = p.leak().long;
    }
    print(*l);
}
// error: `Pair<'a, 'a>` says two lifetimes are the same, but an impl of a trait must hold at every lifetime, as finding the impl for a type doesn't look at lifetimes
