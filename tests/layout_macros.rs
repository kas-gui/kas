use kas::Widget;

fn use_widget<W: Widget<Data = ()>>(_: W) {}

#[test]
fn column() {
    use_widget(kas::column!["one", "two",])
}

#[test]
fn row() {
    use_widget(kas::row!["one", "two"]);
}

#[test]
fn list() {
    use_widget(kas::list!(left, ["one", "two"]));
}

#[test]
fn float() {
    use_widget(kas::float![
        pack!(left top, "one"),
        pack!(right bottom, "two"),
        "some text\nin the\nbackground"
    ]);
}

#[test]
fn grid() {
    use_widget(kas::grid! {
        (0, 0) => "top left",
        (1, 0) => "top right",
        (0..2, 1) => "bottom row (merged)",
    });
}

#[test]
fn aligned_column() {
    use_widget(kas::aligned_column![row!["one", "two"], row![
        "three", "four"
    ],]);
}

#[test]
fn aligned_row() {
    use_widget(kas::aligned_row![column!["one", "two"], column![
        "three", "four"
    ],]);
}

#[test]
fn pack() {
    use_widget(kas::pack!(right top, "132"));
}

#[test]
fn margins() {
    use_widget(kas::margins!(1.0 em, "abc"));
    use_widget(kas::margins!(vert = none, "abc"));
}
