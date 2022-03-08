use kas_macros::autoimpl;
use std::fmt::Debug;
use std::marker::PhantomData;

fn test_has_clone(_: impl Clone) {}
fn test_has_debug(_: impl Debug) {}

#[autoimpl(Clone, Debug where T: trait)]
struct Wrapper<T>(pub T);

#[test]
fn wrapper() {
    test_has_clone(Wrapper(0i32));
    test_has_debug(Wrapper(()));
}

#[autoimpl(Clone where A: Clone, B: Clone)]
#[autoimpl(Debug where A: Debug)]
struct X<A, B: Debug, C> {
    a: A,
    b: B,
    c: PhantomData<C>,
}

#[test]
fn x() {
    let x = X {
        a: 1i8,
        b: "abc",
        c: PhantomData::<fn()>,
    };
    test_has_debug(x.clone());
}
