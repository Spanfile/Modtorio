//! Provides the `StrExt` trait, which provides functions commonly used on strings.

/// Provides functions commonly used on strings.
pub trait StrExt {
    /// Returns `Some(self)` if `self` isn't empty, otherwise returns `None`.
    fn map_to_option(self) -> Option<Self>
    where
        Self: Sized;
}

impl StrExt for String {
    fn map_to_option(self) -> Option<Self>
    where
        Self: Sized,
    {
        if self.is_empty() {
            None
        } else {
            Some(self)
        }
    }
}
