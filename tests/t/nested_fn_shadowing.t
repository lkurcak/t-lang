// A nested `fn` shadows an outer one of the same name, and different
// functions, or blocks, can each have their own.
fn helper() -> i64 { 1 }

fn first() -> i64 {
    fn helper() -> i64 { 10 }
    helper()
}

fn second() -> i64 {
    fn helper() -> i64 { 20 }
    helper()
}

fn main() {
    print(helper()); // expect: 1
    print(first());  // expect: 10
    print(second()); // expect: 20
    {
        // Shadows the top-level `helper` in all of this block, even before.
        print(helper()); // expect: 30
        fn helper() -> i64 { 30 }
        {
            fn helper() -> i64 { 40 }
            print(helper()); // expect: 40
        }
        print(helper()); // expect: 30
    }
    {
        fn helper() -> i64 { 50 }
        print(helper()); // expect: 50
    }
    print(helper()); // expect: 1

    // A local declared before the block is shadowed by the block's `fn`,
    // and one declared in the block shadows it.
    let helper: i64 = 60;
    {
        print(helper()); // expect: 30
        fn helper() -> i64 { 30 }
        let helper: i64 = 70;
        print(helper); // expect: 70
    }
    print(helper); // expect: 60
}
