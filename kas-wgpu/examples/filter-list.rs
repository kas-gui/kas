// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter list example

use kas::widget::view::{ListView, SharedConst};
use kas::widget::Window;

const MONTHS: &[&str] = &[
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let data: &SharedConst<[&str]> = MONTHS.into();
    let window = Window::new("Filter-list", ListView::<kas::Down, _>::new(data));

    let theme = kas_theme::ShadedTheme::new();
    kas_wgpu::Toolkit::new(theme)?.with(window)?.run()
}
