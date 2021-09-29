// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget macros

/// Sugar for `Row::new(vec![...])`
///
/// See [`vec`], [`Row`](crate::Row), [`List`](crate::List).
#[macro_export]
macro_rules! row {
    () => {
        $crate::Row::new(vec![])
    };
    ($item:expr; $n:expr) => {
        $crate::Row::new(vec![$item; $n])
    };
    ($($item:expr),+ $(,)?) => {
        $crate::Row::new(vec![$($item),*])
    };
}

/// Sugar for `Column::new(vec![...])`
///
/// See also [`vec`], [`Column`](crate::Column), [`List`](crate::List).
#[macro_export]
macro_rules! column {
    () => {
        $crate::Column::new(vec![])
    };
    ($item:expr; $n:expr) => {
        $crate::Column::new(vec![$item; $n])
    };
    ($($item:expr),+ $(,)?) => {
        $crate::Column::new(vec![$($item),*])
    };
}
