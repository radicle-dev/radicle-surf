pub mod path {
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
