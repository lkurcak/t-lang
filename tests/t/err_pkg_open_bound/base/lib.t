export trait A {}
export trait B {}
export struct W<I> { export i: I }
impl<I: Iterator> A for W<I> where I::Item: B {} // error: this impl's bound `I::Item: B` isn't on a type parameter of `W<I>`
