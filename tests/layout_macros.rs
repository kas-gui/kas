use kas::Widget;
use kas::layout::AlignHints;
use kas::widgets::{column, float, grid, list, row};

fn use_widget<W: Widget<Data = ()>>(_: W) {}

#[test]
fn column() {
    use_widget(column!["one", "two",])
}

#[test]
fn row() {
    use_widget(row!["one", "two"]);
}

#[test]
fn list() {
    use_widget(list!["one", "two"].with_direction(kas::dir::Left));
}

#[test]
fn float() {
    use_widget(float![
        "one".pack(AlignHints::TOP_LEFT),
        "two".pack(AlignHints::BOTTOM_RIGHT),
        "some text\nin the\nbackground",
    ]);
}

#[test]
fn grid() {
    use_widget(grid! {
        (0, 0) => "top left",
        (1, 0) => "top right",
        (0..=1, 1) => "bottom row (merged)",
    });
}

#[test]
fn aligned_column() {
    #[rustfmt::skip]
    use_widget(grid![
        row!["one", "two"],
        row!["three", "four"],
    ]);
}

#[test]
fn aligned_row() {
    use_widget(grid![column!["one", "two"], column!["three", "four"],]);
}
