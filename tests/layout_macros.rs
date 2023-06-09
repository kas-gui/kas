#[test]
fn column() {
    let _widget = kas::column!["one", "two",];
}

#[test]
fn row() {
    let _widget = kas::row!["one", "two"];
}

#[test]
fn list() {
    let _widget = kas::list!(left, ["one", "two"]);
}

#[test]
fn float() {
    let _widget = kas::float![
        pack!(left top, "one"),
        pack!(right bottom, "two"),
        "some text\nin the\nbackground"
    ];
}

#[test]
fn grid() {
    let _widget = kas::grid! {
        (0, 0) => "top left",
        (1, 0) => "top right",
        (0..2, 1) => "bottom row (merged)",
    };
}

#[test]
fn aligned_column() {
    let _widget = kas::aligned_column![row!["one", "two"], row!["three", "four"],];
}

#[test]
fn aligned_row() {
    let _widget = kas::aligned_row![column!["one", "two"], column!["three", "four"],];
}

#[test]
fn align() {
    let _a = kas::align!(right, "132");
    let _b = kas::align!(left top, "abc");
}

#[test]
fn pack() {
    let _widget = kas::pack!(right top, "132");
}

#[test]
fn margins() {
    let _a = kas::margins!(1.0 em, "abc");
    let _b = kas::margins!(vert = none, "abc");
}
