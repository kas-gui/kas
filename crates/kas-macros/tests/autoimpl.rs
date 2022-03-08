use kas_macros::autoimpl;
use std::fmt::Debug;

fn test_has_clone(_: impl Clone) {}
fn test_has_debug(_: impl Debug) {}

#[autoimpl(Clone, Debug where T: trait)]
struct Wrapper<T>(pub T);

#[test]
fn wrapper() {
    test_has_clone(Wrapper(0i32));
    test_has_debug(Wrapper(()));
}
