export trait A {}
export trait B {}
export struct W<I> { export i: I }
impl<I: Iterator> A for W<I> where I::Item: B {}
