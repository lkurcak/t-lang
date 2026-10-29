//! Bounded, deterministic soundness checks against an independent finite
//! model. Incompleteness is allowed: only a successful proof is a promise.

use super::*;

const SEEDS: [u64; 2] = [0x7465_7374_5f54, 0xc0ff_ee12_3456_789a];
const CASES_PER_SEED: usize = 128;

struct Generator(u64);

impl Generator {
    fn pick(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((self.0 >> 32) % n as u64) as usize
    }

    fn formula(&mut self, depth: usize, large: bool) -> Formula {
        match if depth == 0 { 0 } else { self.pick(6) } {
            0 => {
                // With x,y in -2..=2 and b in 0..=1, evaluating even
                // three large terms plus the constant fits in i128.
                // Elimination's intermediate products need not fit.
                let big = i128::MAX / 64;
                let coefficients = if large {
                    [-big, -big + 1, -1, 0, 1, big - 1, big]
                } else {
                    [-3, -2, -1, 0, 1, 2, 3]
                };
                let mut terms = Vec::new();
                for v in 0..3 {
                    let c = coefficients[self.pick(coefficients.len())];
                    if c != 0 {
                        terms.push((v, c));
                    }
                }
                let lin = Lin {
                    terms,
                    k: coefficients[self.pick(coefficients.len())],
                };
                if self.pick(2) == 0 {
                    Formula::Le(lin)
                } else {
                    Formula::Eq(lin)
                }
            }
            1 => self.formula(depth - 1, large).not(),
            2 => Formula::and([
                self.formula(depth - 1, large),
                self.formula(depth - 1, large),
            ]),
            3 => Formula::or([
                self.formula(depth - 1, large),
                self.formula(depth - 1, large),
            ]),
            4 => Formula::iff(Formula::holds(2), self.formula(depth - 1, large)),
            _ => [Formula::True, Formula::False][self.pick(2)].clone(),
        }
    }
}

// Evaluate the syntax directly, without normalisation, substitution,
// rounding, or any of the solver's linear-arithmetic helpers.
fn eval(f: &Formula, values: [i128; 3]) -> bool {
    match f {
        Formula::True => true,
        Formula::False => false,
        Formula::Le(l) | Formula::Eq(l) => {
            let value = l
                .terms
                .iter()
                .fold(l.k, |sum, &(v, c)| sum + c * values[v as usize]);
            if matches!(f, Formula::Le(_)) {
                value <= 0
            } else {
                value == 0
            }
        }
        Formula::Not(f) => !eval(f, values),
        Formula::And(fs) => fs.iter().all(|f| eval(f, values)),
        Formula::Or(fs) => fs.iter().any(|f| eval(f, values)),
    }
}

fn counterexample(facts: &[Formula], goal: &Formula) -> Option<[i128; 3]> {
    for x in -2..=2 {
        for y in -2..=2 {
            for b in 0..=1 {
                let values = [x, y, b];
                if facts.iter().all(|f| eval(f, values)) && !eval(goal, values) {
                    return Some(values);
                }
            }
        }
    }
    None
}

#[test]
fn generated_proofs_agree_with_finite_models() {
    let kinds = [
        Kind::Int(Lin::constant(-2), Lin::constant(2)),
        Kind::Bound(-2, 2),
        Kind::Bool,
    ];
    let mut proved = [0; 2];
    let mut disproved = [0; 2];
    let mut arithmetic_limits = 0;
    for (mode, large) in [false, true].into_iter().enumerate() {
        for seed in SEEDS {
            let mut generator = Generator(seed);
            for n in 0..CASES_PER_SEED {
                let mut facts: Vec<_> = (0..generator.pick(4))
                    .map(|_| generator.formula(2, large))
                    .collect();
                let goal = generator.formula(2, large);
                let witness = counterexample(&facts, &goal);
                if witness.is_some() {
                    disproved[mode] += 1;
                }
                // Reverse assumptions as well: the finite model is the
                // same, even when a different elimination path gives up.
                for reversed in [false, true] {
                    if reversed {
                        facts.reverse();
                    }
                    let context = format!(
                        "seed {seed:#x}, case {n}, large {large}, reversed {reversed}\n\
                         facts: {facts:?}\ngoal: {goal:?}\ncounterexample: {witness:?}"
                    );
                    let result = std::panic::catch_unwind(|| valid(&facts, &goal, &kinds))
                        .unwrap_or_else(|panic| {
                            let why = panic
                                .downcast_ref::<String>()
                                .map(String::as_str)
                                .or_else(|| panic.downcast_ref::<&str>().copied())
                                .unwrap_or("non-string panic");
                            panic!("{context}\nsolver panicked: {why}");
                        });
                    if result.is_ok() {
                        assert!(witness.is_none(), "{context}\nproved a false implication");
                        proved[mode] += 1;
                    }
                    if result == Err(Failure::ArithmeticLimit) {
                        arithmetic_limits += 1;
                    }
                }
            }
        }
    }
    // Guard against an accidentally vacuous generator or one that never
    // reaches the checked-arithmetic fallback.
    assert!(proved.into_iter().all(|n| n >= 20), "{proved:?}");
    assert!(disproved.into_iter().all(|n| n >= 20), "{disproved:?}");
    assert!(arithmetic_limits >= 20, "{arithmetic_limits}");
}

#[test]
fn generated_extreme_atoms_never_prove_a_false_implication() {
    // x is 0 or 1, so each atom can be evaluated exactly even at MIN/MAX.
    // Generate both extreme coefficients and extreme constant offsets.
    let kinds = [Kind::Bound(0, 1)];
    let edges = [i128::MIN, i128::MIN + 1, -1, 0, 1, i128::MAX - 1, i128::MAX];
    for edge in edges {
        for (coefficient, constant) in [(edge, 0), (0, edge)] {
            for equality in [false, true] {
                let lin = Lin {
                    terms: if coefficient == 0 {
                        vec![]
                    } else {
                        vec![(0, coefficient)]
                    },
                    k: constant,
                };
                let atom = if equality {
                    Formula::Eq(lin)
                } else {
                    Formula::Le(lin)
                };
                for negated in [false, true] {
                    let fact = if negated {
                        atom.clone().not()
                    } else {
                        atom.clone()
                    };
                    for target in 0..=1 {
                        let goal = Formula::eq(&Lin::var(0), &Lin::constant(target)).unwrap();
                        // A fact satisfying the other x value is a concrete
                        // counterexample to x == target.
                        if eval(&fact, [1 - target, 0, 0]) {
                            assert!(
                                valid(std::slice::from_ref(&fact), &goal, &kinds).is_err(),
                                "fact: {fact:?}, goal: {goal:?}"
                            );
                        }
                        // Exercise extreme atoms in the goal's negation too.
                        if !eval(&fact, [target, 0, 0]) {
                            assert!(
                                valid(&[goal], &fact, &kinds).is_err(),
                                "false goal: {fact:?}, x = {target}"
                            );
                        }
                    }
                }
            }
        }
    }
}
