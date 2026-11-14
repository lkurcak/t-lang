// By `&`, a closure captures through references too: `|| s.step` for an
// `s: &mut S` borrows `(*s).step`, so `s.count` can be borrowed `&mut`
// alongside, as a method's `self.a` and `self.b` would.
#[derive(Discard)]
struct S { count: u64, step: u64, name: (u64, u64) }

impl S {
    fn bump(&mut self) with panic {
        let count = &mut self.count;
        let step = || self.step;
        *count = *count + step();
        *count = *count + step();
    }
}

fn twice(s: &mut S) with panic {
    let c = &mut s.name.0;
    let inner = || s.name.1 + s.step;
    *c = inner();
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {panic, alloc} {
    let mut out = os.stdout;
    let mut s = S { count: 1, step: 3, name: (0, 4) };
    s.bump();
    twice(&mut s);
    out.line(&f"{s.count} {s.name.0}")?; // expect: 7 7
    // Through a `Box`, and through two references.
    let b = alloc::boxed::Box::new(S { count: 2, step: 5, name: (1, 1) });
    let get = || b.count * b.step;
    let r = &&s;
    let read = || r.step;
    out.line(&f"{get()} {read()}") // expect: 10 3
}
