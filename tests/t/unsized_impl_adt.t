// An impl for a struct whose `unsized` parameter is `str`, a trait's and
// its own: `Ref<'a, str>` has a size, as a `&'a str` does, so its methods'
// `Self` may be it, and so may an array's values.
#[derive(Copy, Discard)]
struct Ref<'a, unsized T> { r: &'a T }

trait Show { fn show(&self) -> usize; }

impl<'a> Show for Ref<'a, str> {
    fn show(&self) -> usize { self.r.len() }
}

impl<'a> Ref<'a, str> {
    fn n(self) -> usize { self.r.len() }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let r: Ref<'static, str> = Ref { r: "four" };
    out.line(&r.show())?; // expect: 4
    out.line(&r.n())?; // expect: 4
    let rs: [Ref<'static, str>; 2] = [r, Ref { r: "ab" }];
    out.line(&rs[1].show())?; // expect: 2
    Ok(())
}
