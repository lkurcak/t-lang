// `<`, `<=`, `>` and `>=` are `Ord`'s: they compare anything ordered, and
// look at their operands where they are, as `==` does, moving neither.
use alloc::string::String;
use core::cmp::Ordering;

#[derive(Discard, Eq)]
struct Version { major: u32, minor: u32 }

impl Ord for Version {
    fn cmp(&self, other: &Version) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => self.minor.cmp(&other.minor),
            o => o,
        }
    }
}

fn least<T: Ord + Discard>(a: T, b: T) -> T { if b < a { b } else { a } }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    out.line(&(false < true))?; // expect: true
    out.line(&('a' < 'b'))?;    // expect: true
    out.line(&('é' > 'z'))?;    // expect: true
    out.line(&("abc" < "abd"))?; // expect: true
    out.line(&("ab" >= "abc"))?; // expect: false
    let s = String::from("pear");
    let t = String::from("apple");
    let w = show(&mut out, &s, &t);
    s.discard();
    t.discard();
    w?;
    // expect: true
    // expect: pear
    // expect: apple
    let v1 = Version { major: 1, minor: 9 };
    let v2 = Version { major: 2, minor: 0 };
    out.line(&(v1 < v2))?;  // expect: true
    out.line(&(v2 <= v1))?; // expect: false
    let r1 = &v1;
    let r2 = &v2;
    out.line(&(r1 >= r2))?; // expect: false
    out.line(&least(v1, v2).minor)?; // expect: 9
    out.line(&least(5i64, 3))?; // expect: 3
    Result::Ok(())
}

fn show(out: &mut std::io::Stdout, s: &String, t: &String) -> Result<(), std::io::Error> {
    out.line(&(t < s))?;
    // Neither was moved.
    out.line(&s.as_str())?;
    out.line(&t.as_str())
}
