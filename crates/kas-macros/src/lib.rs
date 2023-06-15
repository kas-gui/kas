// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS macros
//!
//! This crate extends [`impl-tools`](https://docs.rs/impl-tools/).

extern crate proc_macro;

use impl_tools_lib::{self as lib, autoimpl};
use proc_macro::TokenStream;
use proc_macro_error::{emit_call_site_error, proc_macro_error};
use syn::spanned::Spanned;
use syn::{parse_macro_input, parse_quote_spanned};

mod class_traits;
mod extends;
mod make_layout;
mod widget;
mod widget_index;

/// Implement `Default`
///
/// See [`impl_tools::impl_default`](https://docs.rs/impl-tools/0.6/impl_tools/attr.impl_default.html)
/// for full documentation.
#[proc_macro_attribute]
#[proc_macro_error]
pub fn impl_default(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut toks = item.clone();
    match syn::parse::<lib::ImplDefault>(attr) {
        Ok(attr) => toks.extend(TokenStream::from(attr.expand(item.into()))),
        Err(err) => {
            emit_call_site_error!(err);
            // Since this form of invocation only adds implementations, we can
            // safely output the original item, thus reducing secondary errors.
        }
    }
    toks
}

/// A variant of the standard `derive` macro
///
/// See [`impl_tools::autoimpl`](https://docs.rs/impl-tools/0.6/impl_tools/attr.autoimpl.html)
/// for full documentation.
///
/// The following traits are supported:
///
/// | Path | *ignore* | *using* | *notes* |
/// |----- |--- |--- |--- |
/// | [`::core::borrow::Borrow<T>`] | - | borrow target | `T` is type of target field |
/// | [`::core::borrow::BorrowMut<T>`] | - | borrow target | `T` is type of target field |
/// | [`::core::clone::Clone`] | yes | - | ignored fields use `Default::default()` |
/// | [`::core::cmp::Eq`] | * | - | *allowed with `PartialEq` |
/// | [`::core::cmp::Ord`] | yes | - | |
/// | [`::core::cmp::PartialEq`] | yes | - | |
/// | [`::core::cmp::PartialOrd`] | yes | - | |
/// | [`::core::convert::AsRef<T>`] | - | ref target | `T` is type of target field |
/// | [`::core::convert::AsMut<T>`] | - | ref target | `T` is type of target field |
/// | [`::core::default::Default`] | - | - | [`macro@impl_default`] is a more flexible alternative |
/// | [`::core::fmt::Debug`] | yes | - | |
/// | [`::core::hash::Hash`] | yes | - | |
/// | [`::core::marker::Copy`] | * | - | *allowed with `Clone` |
/// | [`::core::ops::Deref`] | - | deref target | `type Target` is type of target field |
/// | [`::core::ops::DerefMut`] | - | deref target | `type Target` is type of target field |
/// | `::kas::class::HasBool` | - | deref target | |
/// | `::kas::class::HasStr` | - | deref target | |
/// | `::kas::class::HasString` | - | deref target | |
/// | `::kas::class::SetAccel` | - | deref target | |
/// | `class_traits` | - | deref target | implements each `kas::class` trait |
///
/// ### Examples
///
/// Implement all `kas::class` trait over a wrapper type:
/// ```no_test
/// # use kas_macros::autoimpl;
/// #[autoimpl(class_traits using self.0 where T: trait)]
/// struct WrappingType<T>(T);
/// ```
#[proc_macro_attribute]
#[proc_macro_error]
pub fn autoimpl(attr: TokenStream, item: TokenStream) -> TokenStream {
    use autoimpl::ImplTrait;
    use class_traits::ImplClassTraits;
    use std::iter::once;

    let mut toks = item.clone();
    match syn::parse::<autoimpl::Attr>(attr) {
        Ok(autoimpl::Attr::ForDeref(ai)) => toks.extend(TokenStream::from(ai.expand(item.into()))),
        Ok(autoimpl::Attr::ImplTraits(ai)) => {
            // We could use lazy_static to construct a HashMap for fast lookups,
            // but given the small number of impls a "linear map" is fine.
            let find_impl = |path: &syn::Path| {
                (autoimpl::STD_IMPLS.iter())
                    .chain(class_traits::CLASS_IMPLS.iter())
                    .cloned()
                    .chain(once(&ImplClassTraits as &dyn ImplTrait))
                    .find(|impl_| impl_.path().matches_ident_or_path(path))
            };
            toks.extend(TokenStream::from(ai.expand(item.into(), find_impl)))
        }
        Err(err) => {
            emit_call_site_error!(err);
            // Since autoimpl only adds implementations, we can safely output
            // the original item, thus reducing secondary errors.
        }
    }
    toks
}

const IMPL_SCOPE_RULES: [&dyn lib::ScopeAttr; 2] = [&lib::AttrImplDefault, &widget::AttrImplWidget];

fn find_attr(path: &syn::Path) -> Option<&'static dyn lib::ScopeAttr> {
    IMPL_SCOPE_RULES
        .iter()
        .cloned()
        .find(|rule| rule.path().matches(path))
}

/// Scope supporting `impl Self` and advanced attribute macros
///
/// This macro facilitates definition of a type (struct, enum or union) plus
/// implementations via `impl Self { .. }` syntax: `Self` is expanded to the
/// type's name, including generics and bounds (as defined on the type).
///
/// Caveat: `rustfmt` can not yet format contents (see
/// [rustfmt#5254](https://github.com/rust-lang/rustfmt/issues/5254),
/// [rustfmt#5538](https://github.com/rust-lang/rustfmt/pull/5538)).
///
/// See [`impl_tools::impl_scope`](https://docs.rs/impl-tools/0.6/impl_tools/macro.impl_scope.html)
/// for full documentation.
///
/// ## Special attribute macros
///
/// Additionally, `impl_scope!` supports special attribute macros evaluated
/// within its scope:
///
/// -   [`#[impl_default]`](macro@impl_default): implement [`Default`] using
///     field initializers (which are not legal syntax outside of `impl_scope!`)
/// -   [`#[widget]`](macro@widget): implement `kas::Widget` trait family
///
/// Note: matching these macros within `impl_scope!` does not use path
/// resolution. Using `#[impl_tools::impl_default]` would resolve the variant
/// of this macro which *doesn't support* field initializers.
#[proc_macro_error]
#[proc_macro]
pub fn impl_scope(input: TokenStream) -> TokenStream {
    let mut scope = parse_macro_input!(input as lib::Scope);
    scope.apply_attrs(find_attr);
    scope.expand().into()
}

/// Attribute to implement the `kas::Widget` family of traits
///
/// This may *only* be used within the [`impl_scope!`] macro.
///
/// Implements the [`WidgetCore`] and [`Widget`] traits for the deriving type.
/// Implements the [`WidgetChildren`] and [`Layout`]
/// traits only if not implemented explicitly within the
/// defining [`impl_scope!`].
///
/// This macro may inject methods into existing [`Layout`] / [`Widget`] implementations.
/// This is used both to provide default implementations which could not be
/// written on the trait and to implement properties like `navigable`.
/// (In the case of multiple implementations of the same trait, as used for
/// specialization, only the first implementation of each trait is extended.)
///
/// ## Syntax
///
/// > _WidgetAttr_ :\
/// > &nbsp;&nbsp; `#` `[` `widget` _WidgetAttrArgs_? `]`
/// >
/// > _WidgetAttrArgs_ :\
/// > &nbsp;&nbsp; `{` (_WidgetAttrArg_ `;`) * `}`
///
/// Supported arguments (_WidgetAttrArg_) are:
///
/// -   <code>derive = self.<em>field</em></code> where
///     <code><em>field</em></code> is the name (or number) of a field:
///     enables "derive mode" ([see below](#derive)) over the given field
/// -   <code>navigable = <em>bool</em></code> — a quick implementation of
///     `Widget::navigable`: whether this widget supports keyboard focus via
///     the <kbd>Tab</kbd> key (default is `false`)
/// -   <code>hover_highlight = <em>bool</em></code> — if true, then match
///     `Event::MouseHover` and `Event::LostMouseHover`, requesting redraw and
///     returning `Response::Used`
/// -   <code>cursor_icon = <em>expr</em></code> — if used, then match
///     `Event::MouseHover`, calling `mgr.set_cursor_icon(expr)`
/// -   <code>layout = <em>layout</em></code> — defines widget layout via an
///     expression; [see below for documentation](#layout)
///
/// The struct must contain a field of type `widget_core!()` (usually named
/// `core`). The macro `widget_core!()` is a placeholder, expanded by
/// `#[widget]` and used to identify the field used (name may be anything).
/// This field *may* have type [`CoreData`] or may be a generated
/// type; either way it has fields `id: WidgetId` (assigned by
/// `Widget::pre_configure`) and `rect: Rect` (usually assigned by
/// `Widget::set_rect`). It may contain additional fields for layout data. The
/// type supports `Default` and `Clone` (although `Clone` actually
/// default-initializes all fields other than `rect` since clones of widgets
/// must themselves be configured).
///
/// Assuming the deriving type is a `struct` or `tuple struct`, fields support
/// the following attributes:
///
/// -   `#[widget]`: marks the field as a [`Widget`] to be configured, enumerated by
///     [`WidgetChildren`] and included by glob layouts
///
/// ## Layout
///
/// Widget layout may be specified either by implementing the `Layout` trait or
/// via the `layout` property of `#[widget]`. The latter accepts the following
/// syntax, where _Layout_ is any of the below.
///
/// Using the `layout = ...;` property will also generate a corresponding
/// implementation of `Widget::nav_next`, with a couple of exceptions
/// (where macro-time analysis is insufficient to implement this method).
///
/// > [_Column_](macro@column), [_Row_](macro@row), [_List_](macro@list), [_AlignedColumn_](macro@aligned_column), [_AlignedRow_](macro@aligned_row), [_Grid_](macro@grid), [_Float_](macro@float), [_Align_](macro@align), [_Pack_](macro@pack), [_Margins_](macro@margins) :\
/// > &nbsp;&nbsp; These stand-alone macros are explicitly supported in this position.\
/// > &nbsp;&nbsp; Optionally, a _Storage_ specifier is supported immediately after the macro name, e.g.\
/// > &nbsp;&nbsp; `column! 'storage_name ["one", "two"]`
///
/// > _Single_ :\
/// > &nbsp;&nbsp; `self` `.` _Member_\
/// > &nbsp;&nbsp; A named child: `self.foo` (more precisely, this matches any expression starting `self`, and uses `&mut (#expr)`)
/// >
/// > _Slice_ :\
/// > &nbsp;&nbsp; `slice!` _Storage_? `(` _Direction_ `,` `self` `.` _Member_ `)`\
/// > &nbsp;&nbsp; A field with type `[W]` for some `W: Layout`. (Note: this does not automatically register the slice widgets as children for the purpose of configuration and event-handling. An explicit implementation of `WidgetChildren` will be required.)
/// >
/// > _Frame_ :\
/// > &nbsp;&nbsp; `frame!` _Storage_? `(` _Layout_ ( `,` `style` `=` _Expr_ )? `)`\
/// > &nbsp;&nbsp; Adds a frame of type _Expr_ around content, defaulting to `FrameStyle::Frame`.
/// >
/// > _Button_ :\
/// > &nbsp;&nbsp; `button!` _Storage_? `(` _Layout_ ( `,` `color` `=` _Expr_ )? `)`\
/// > &nbsp;&nbsp; Adds a button frame (optionally with color _Expr_) around content.
/// >
/// > _WidgetConstructor_ :\
/// > &nbsp;&nbsp; _Expr_\
/// > &nbsp;&nbsp; An expression yielding a widget, e.g. `Label::new("Hello world")`. The result must be an object of some type `W: Widget`.
/// >
/// > _LabelLit_ :\
/// > &nbsp;&nbsp; _StrLit_\
/// > &nbsp;&nbsp; A string literal generates a label widget, e.g. "Hello world". This is an internal type without text wrapping.
/// >
/// > _NonNavigable_ :\
/// > &nbsp;&nbsp; `non_navigable!` `(` _Layout_ `)` \
/// > &nbsp;&nbsp; Does not affect layout. Specifies that the content is excluded from tab-navigation order.
///
/// Additional syntax rules (not layout items):
///
/// > _Member_ :\
/// > &nbsp;&nbsp; _Ident_ | _Index_\
/// > &nbsp;&nbsp; The name of a struct field or an index into a tuple struct.
/// >
/// > _Direction_ :\
/// > &nbsp;&nbsp; `left` | `right` | `up` | `down` | _Expr_:\
/// > &nbsp;&nbsp; Note that an _Expr_ must start with `self`
/// >
/// > _Storage_ :\
/// > &nbsp;&nbsp; `'` _Ident_\
/// > &nbsp;&nbsp; Used to explicitly name the storage used by a generated widget or layout; for example `row 'x: ["A", "B", "C"]` will add a field `x: R` where `R: RowStorage` within the generated `widget_core!()`. If omitted, the field name will be anonymous (generated).
///
/// ## Examples
///
/// A simple example is the
/// [`Frame`](https://docs.rs/kas-widgets/latest/kas_widgets/struct.Frame.html) widget:
///
/// ```ignore
/// impl_scope! {
///     /// A frame around content
///     #[autoimpl(Deref, DerefMut using self.inner)]
///     #[autoimpl(class_traits using self.inner where W: trait)]
///     #[derive(Clone, Default)]
///     #[widget{
///         layout = frame!(self.inner, style = kas::theme::FrameStyle::Frame);
///     }]
///     pub struct Frame<W: Widget> {
///         core: widget_core!(),
///         #[widget]
///         pub inner: W,
///     }
///
///     impl Self {
///         /// Construct a frame
///         #[inline]
///         pub fn new(inner: W) -> Self {
///             Frame {
///                 core: Default::default(),
///                 inner,
///             }
///         }
///     }
/// }
/// ```
///
/// A simple row layout: `layout = row! [self.a, self.b];`
///
/// ## Derive
///
/// It is possible to derive from a field which is itself a widget, e.g.:
/// ```ignore
/// impl_scope! {
///     #[autoimpl(Deref, DerefMut using self.0)]
///     #[derive(Clone, Default)]
///     #[widget{ derive = self.0; }]
///     pub struct ScrollBarRegion<W: Widget>(ScrollBars<ScrollRegion<W>>);
/// }
/// ```
///
/// This is a special mode where most features of `#[widget]` are not
/// available. A few may still be used: `navigable`, `hover_highlight`,
/// `cursor_icon`. Additionally, it is currently permitted to implement
/// [`WidgetChildren`], [`Layout`] and [`Widget`] traits manually (this option
/// may be removed in the future if not deemed useful).
///
/// [`Widget`]: https://docs.rs/kas/0.11/kas/trait.Widget.html
/// [`WidgetCore`]: https://docs.rs/kas/0.11/kas/trait.WidgetCore.html
/// [`WidgetChildren`]: https://docs.rs/kas/0.11/kas/trait.WidgetChildren.html
/// [`Layout`]: https://docs.rs/kas/0.11/kas/trait.Layout.html
/// [`CursorIcon`]: https://docs.rs/kas/0.11/kas/event/enum.CursorIcon.html
/// [`Response`]: https://docs.rs/kas/0.11/kas/event/enum.Response.html
/// [`CoreData`]: https://docs.rs/kas/0.11/kas/struct.CoreData.html
#[proc_macro_attribute]
#[proc_macro_error]
pub fn widget(_: TokenStream, item: TokenStream) -> TokenStream {
    emit_call_site_error!("must be used within impl_scope! { ... }");
    item
}

/// Construct a single-instance struct
///
/// Rust doesn't currently support [`impl Trait { ... }` expressions](https://github.com/canndrew/rfcs/blob/impl-trait-expressions/text/0000-impl-trait-expressions.md)
/// or implicit typing of struct fields. This macro is a **hack** allowing that.
///
/// Example:
/// ```
/// # use kas_macros as kas;
/// use std::fmt;
/// fn main() {
///     let world = "world";
///     let says_hello_world = kas::singleton! {
///         struct(&'static str = world);
///         impl fmt::Display for Self {
///             fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
///                 write!(f, "hello {}", self.0)
///             }
///         }
///     };
///     assert_eq!(format!("{}", says_hello_world), "hello world");
/// }
/// ```
///
/// That is, this macro creates an anonymous struct type (must be a struct),
/// which may have trait implementations, then creates an instance of that
/// struct.
///
/// Struct fields may have a fixed type or may be generic. Syntax is as follows:
///
/// -   **regular struct:** `ident: ty = value`
/// -   **regular struct:** `ident: ty` (uses `Default` to construct value)
/// -   **regular struct:** `ident = value` (type is generic without bounds)
/// -   **tuple struct:** `ty = value`
/// -   **tuple struct:** `ty` (uses `Default` to construct value)
///
/// The field name, `ident`, may be `_` (anonymous field).
///
/// The field type, `ty`, may be or may contain inferred types (`_`) and/or
/// `impl Trait` type expressions. These are substituted with generics on the
/// type. In the special case that the field has attribute `#[widget]` and type
/// `_`, the generic for this type has bound `::kas::Widget` applied.
///
/// Refer to [examples](https://github.com/search?q=singleton+repo%3Akas-gui%2Fkas+path%3Aexamples&type=Code) for usage.
#[proc_macro_error]
#[proc_macro]
pub fn singleton(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as lib::Singleton);
    for field in input.fields.iter_mut() {
        if let syn::Type::Infer(token) = &field.ty {
            field.ty = parse_quote_spanned! {token.span()=>
                impl ::kas::Widget
            };
        }
    }
    let mut scope = input.into_scope();
    scope.apply_attrs(find_attr);
    scope.expand().into()
}

/// Index of a child widget
///
/// This macro is usable only within an [`impl_scope!`]  macro using the
/// [`widget`](macro@widget) attribute.
///
/// Example usage: `widget_index![self.a]`. If `a` is a child widget (a field
/// marked with the `#[widget]` attribute), then this expands to the child
/// widget's index (as used by [`WidgetChildren`]). Otherwise, this is an error.
///
/// [`WidgetChildren`]: https://docs.rs/kas/0.11/kas/trait.WidgetChildren.html
#[proc_macro_error]
#[proc_macro]
pub fn widget_index(input: TokenStream) -> TokenStream {
    let input2 = input.clone();
    let _ = parse_macro_input!(input2 as widget_index::BaseInput);
    input
}

trait ExpandLayout {
    fn expand_layout(self, name: &str) -> TokenStream;
}
impl ExpandLayout for make_layout::Tree {
    fn expand_layout(self, name: &str) -> TokenStream {
        match self.expand_as_widget(name) {
            Ok(toks) => toks.into(),
            Err(err) => {
                emit_call_site_error!(err);
                TokenStream::default()
            }
        }
    }
}

/// Make a column widget
///
/// Items support [widget layout syntax](macro@widget#layout-1).
///
/// # Example
///
/// ```ignore
/// let my_widget = kas::column! [
///     "one",
///     "two",
/// ];
/// ```
#[proc_macro_error]
#[proc_macro]
pub fn column(input: TokenStream) -> TokenStream {
    parse_macro_input!(input with make_layout::Tree::column).expand_layout("_Column")
}

/// Make a row widget
///
/// Items support [widget layout syntax](macro@widget#layout-1).
///
/// # Example
///
/// ```ignore
/// let my_widget = kas::row! ["one", "two"];
/// ```
#[proc_macro_error]
#[proc_macro]
pub fn row(input: TokenStream) -> TokenStream {
    parse_macro_input!(input with make_layout::Tree::row).expand_layout("_Row")
}

/// Make a list widget
///
/// This is a more generic variant of [`column!`] and [`row!`].
///
/// Children are navigated in visual order.
///
/// Items support [widget layout syntax](macro@widget#layout-1).
///
/// # Example
///
/// ```ignore
/// let my_widget = kas::list!(left, ["one", "two"]);
/// ```
///
/// # Syntax
///
/// > _List_ :\
/// > &nbsp;&nbsp; `list!` `(` _Direction_ `,` `[` ( _Layout_ `,` )* ( _Layout_ `,`? )? `]` `}`
/// >
/// > _Direction_ :\
/// > &nbsp;&nbsp; `left` | `right` | `up` | `down`

#[proc_macro_error]
#[proc_macro]
pub fn list(input: TokenStream) -> TokenStream {
    parse_macro_input!(input with make_layout::Tree::list).expand_layout("_List")
}

/// Make a float widget
///
/// All children occupy the same space with the first child on top.
///
/// Size is determined as the maximum required by any child for each axis.
/// All children are assigned this size. It is usually necessary to use [`pack!`]
/// or a similar mechanism to constrain a child to avoid it hiding the content
/// underneath (note that even if an unconstrained child does not *visually*
/// hide everything beneath, it may still "occupy" the assigned area, preventing
/// mouse clicks from reaching the widget beneath).
///
/// Children are navigated in order of declaration.
///
/// Items support [widget layout syntax](macro@widget#layout-1).
///
/// # Example
///
/// ```ignore
/// let my_widget = kas::float! [
///     pack!(left top, "one"),
///     pack!(right bottom, "two"),
///     "some text\nin the\nbackground"
/// ];
/// ```
#[proc_macro_error]
#[proc_macro]
pub fn float(input: TokenStream) -> TokenStream {
    parse_macro_input!(input with make_layout::Tree::float).expand_layout("_Float")
}

/// Make a grid widget
///
/// Constructs a table with auto-determined number of rows and columns.
/// Each child is assigned a cell using match-like syntax.
///
/// A child may be stretched across multiple cells using range-like syntax:
/// `3..5`, `3..=4` and `3..+2` are all equivalent.
///
/// Behaviour of overlapping widgets is identical to [`float!`]: the first
/// declared item is on top.
///
/// Children are navigated in order of declaration.
///
/// Items support [widget layout syntax](macro@widget#layout-1).
///
/// # Example
///
/// ```ignore
/// let my_widget = kas::grid! {
///     (0, 0) => "top left",
///     (1, 0) => "top right",
///     (0..2, 1) => "bottom row (merged)",
/// };
/// ```
///
/// # Syntax
///
/// > _Grid_ :\
/// > &nbsp;&nbsp; `grid!` `{` _GridCell_* `}`
/// >
/// > _GridCell_ :\
/// > &nbsp;&nbsp; `(` _CellRange_ `,` _CellRange_ `)` `=>` ( _Layout_ | `{` _Layout_ `}` )
/// >
/// > _CellRange_ :\
/// > &nbsp;&nbsp; _LitInt_ ( `..` `+`? _LitInt_ )?
///
/// Cells are specified using `match`-like syntax from `(col_spec, row_spec)` to
/// a layout, e.g.: `(1, 0) => self.foo`. Spans are specified via range syntax,
/// e.g. `(0..2, 1) => self.bar`.
#[proc_macro_error]
#[proc_macro]
pub fn grid(input: TokenStream) -> TokenStream {
    parse_macro_input!(input with make_layout::Tree::grid).expand_layout("_Grid")
}

/// Make an aligned column widget
///
/// Items support [widget layout syntax](macro@widget#layout-1).
///
/// # Example
///
/// ```ignore
/// let my_widget = kas::aligned_column! [
///     row!["one", "two"],
///     row!["three", "four"],
/// ];
/// ```
#[proc_macro_error]
#[proc_macro]
pub fn aligned_column(input: TokenStream) -> TokenStream {
    parse_macro_input!(input with make_layout::Tree::aligned_column).expand_layout("_AlignedColumn")
}

/// Make an aligned row widget
///
/// Items support [widget layout syntax](macro@widget#layout-1).
///
/// # Example
///
/// ```ignore
/// let my_widget = kas::aligned_row! [
///     column!["one", "two"],
///     column!["three", "four"],
/// ];
/// ```
#[proc_macro_error]
#[proc_macro]
pub fn aligned_row(input: TokenStream) -> TokenStream {
    parse_macro_input!(input with make_layout::Tree::aligned_row).expand_layout("_AlignedRow")
}

/// Make an align widget
///
/// This is a small wrapper which adjusts the alignment of its contents.
///
/// The alignment specifier may be one or two keywords (space-separated,
/// horizontal component first): `default`, `center`, `stretch`, `left`,
/// `right`, `top`, `bottom`.
///
/// # Example
///
/// ```ignore
/// let a = kas::align!(right, "132");
/// let b = kas::align!(left top, "abc");
/// ```
#[proc_macro_error]
#[proc_macro]
pub fn align(input: TokenStream) -> TokenStream {
    parse_macro_input!(input with make_layout::Tree::align).expand_layout("_Align")
}

/// Make a pack widget
///
/// This is a small wrapper which adjusts the alignment of its contents and
/// prevents its contents from stretching.
///
/// The alignment specifier may be one or two keywords (space-separated,
/// horizontal component first): `default`, `center`, `stretch`, `left`,
/// `right`, `top`, `bottom`.
///
/// # Example
///
/// ```ignore
/// let my_widget = kas::pack!(right top, "132");
/// ```
#[proc_macro_error]
#[proc_macro]
pub fn pack(input: TokenStream) -> TokenStream {
    parse_macro_input!(input with make_layout::Tree::pack).expand_layout("_Pack")
}

/// Make a margin-adjustment widget wrapper
///
/// This is a small wrapper which adjusts the margins of its contents.
///
/// # Example
///
/// ```ignore
/// let a = kas::margins!(1.0 em, "abc");
/// let b = kas::margins!(vert = none, "abc");
/// ```
///
/// # Syntax
///
/// The macro takes one of two forms:
///
/// > _Margins_:\
/// > &nbsp;&nbsp; `margins!` `(` _MarginSpec_ `,` _Layout_ `)`\
/// > &nbsp;&nbsp; `margins!` `(` _MarginDirection_ `=` _MarginSpec_ `,` _Layout_ `)`\
/// >
/// > _MarginDirection_ :\
/// > &nbsp;&nbsp; `horiz` | `horizontal` | `vert` | `vertical` | `left` | `right` | `top` | `bottom`
/// >
/// > _MarginSpec_ :\
/// > &nbsp;&nbsp; ( _LitFloat_ `px` ) | ( _LitFloat_ `em` ) | `none` | `inner` | `tiny` | `small` | `large` | `text`
///
/// | _MarginSpec_ | Description (guide size; theme-specified sizes may vary and may not scale linearly) |
/// | --- | --- |
/// | `none` | No margin (`0px`) |
/// | `inner` | A very tiny theme-specified margin sometimes used to draw selection outlines (`1px`) |
/// | `tiny` | A very small theme-specified margin (`2px`) |
/// | `small` | A small theme-specified margin (`4px`) |
/// | `large`| A large theme-specified margin (`7px`) |
/// | `text` | Text-specific margins; often asymmetric |
/// | _LitFloat_ `em` (e.g. `1.2 em`) | Using the typographic unit Em (`1Em` is the text height, excluding ascender and descender) |
/// | _LitFloat_ `px` (e.g. `5.0 px`) | Using virtual pixels (affected by the scale factor) |
#[proc_macro_error]
#[proc_macro]
pub fn margins(input: TokenStream) -> TokenStream {
    parse_macro_input!(input with make_layout::Tree::margins).expand_layout("_Margins")
}

/// A trait implementation is an extension over some base
///
/// Usage as follows:
/// ```ignore
/// #[extends(ThemeDraw, base = self.base())]
/// impl ThemeDraw for Object {
///     // All methods not present are implemented automatically over
///     // `self.base()`, which mut return an object implementing ThemeDraw
/// }
/// ```
///
/// Note: this is a very limited macro which *only* supports `ThemeDraw`.
#[proc_macro_attribute]
#[proc_macro_error]
pub fn extends(attr: TokenStream, item: TokenStream) -> TokenStream {
    match syn::parse::<extends::Extends>(attr) {
        Ok(extends) => match extends.extend(item.into()) {
            Ok(result) => result.into(),
            Err(err) => {
                emit_call_site_error!(err);
                TokenStream::new()
            }
        },
        Err(err) => {
            emit_call_site_error!(err);
            item
        }
    }
}
