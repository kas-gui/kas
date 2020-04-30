// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text processing

/// Convenience definition: `Cow<'a, str>`
pub type CowStringL<'a> = std::borrow::Cow<'a, str>;

/// Convenience definition: `Cow<'static, str>`
pub type CowString = CowStringL<'static>;
