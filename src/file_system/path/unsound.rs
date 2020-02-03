//! In Irish slang there exists the term "sound". One is a "sound" person if they are nice and you
//! can rely on them. This module is the anithesis of being "sound", you might say it is "unsound".
//!
//! The aim of this module is to make testing easier. During test time, _we know_ that a string is
//! going to be non-empty because we are using the literal `"sound_label"`. The same for knowing
//! that the form `"what/a/sound/bunch"` is a valid path.
//!
//! On the other hand, if we do not control the data coming in we should use the more "sound"
//! method of the [`std::convert::TryFrom`] instance for [`crate::file_system::path::Label`] and
//! [`crate::file_system::path::Path`] to ensure we have valid data to use for further
//! operations.

pub mod path {
    //! Unsound creation of [`Path`]s.

    use crate::file_system::path::Path;
    use std::convert::TryFrom;

    /// **NB**: Use with caution!
    ///
    /// Calls `try_from` on the input and expects it to not fail.
    ///
    /// Used for testing and playground purposes.
    pub fn new(path: &str) -> Path {
        Path::try_from(path).expect("unsafe_path: Failed to parse path")
    }
}

pub mod label {
    //! Unsound creation of [`Label`]s.

    use crate::file_system::path::Label;
    use std::convert::TryFrom;

    /// **NB**: Use with caution!
    ///
    /// Calls `try_from` on the input and expects it to not fail.
    ///
    /// Used for testing and playground purposes.
    pub fn new(path: &str) -> Label {
        Label::try_from(path).expect("unsafe_path: Failed to parse label")
    }
}
