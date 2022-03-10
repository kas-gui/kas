use kas_macros::autoimpl;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::DerefMut;

fn test_has_clone(_: impl Clone) {}
fn test_has_debug(_: impl Debug) {}

#[autoimpl(Clone, Debug where T: trait)]
struct Wrapper<T>(pub T);

#[test]
fn wrapper() {
    test_has_clone(Wrapper(0i32));
    test_has_debug(Wrapper(()));
}

#[autoimpl(Clone, Default where A: trait, B: trait)]
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

#[autoimpl(Deref, DerefMut on self.t)]
struct Y<S, T> {
    _s: S,
    t: T,
}

#[test]
fn y() {
    let mut y = Y { _s: (), t: 1i32 };

    fn set(x: &mut i32) {
        *x = 2;
    }
    set(y.deref_mut());

    assert_eq!(y.t, 2);
}

#[autoimpl(for<'a, T: trait> &'a mut T, Box<T>)]
trait Z {
    const A: i32;

    fn f(&self);
    fn g(&mut self, a: i32, b: &Self::B);

    type B;
}

impl Z for () {
    const A: i32 = 10;

    fn f(&self) {}
    fn g(&mut self, _: i32, _: &i32) {}

    type B = i32;
}

#[test]
fn z() {
    fn impls_z(_: impl Z) {}

    impls_z(());
    impls_z(&mut ());
    impls_z(Box::new(()));
}

#[autoimpl(for<'a, V, T> &'a T, &'a mut T where T: trait + ?Sized)]
#[autoimpl(for<V> Box<dyn G<V>> using dyn G<V>)]
trait G<V>
where
    V: Debug,
{
    fn g(&self) -> V;
}
