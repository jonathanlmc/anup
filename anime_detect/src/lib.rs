mod parse;

pub mod episode;
pub mod series;

pub use episode::Episode;
pub use series::Series;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("input could not be matched")]
    Unmatched,
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait Format: Sized + Copy + 'static {
    const VARIANT_MAPPINGS: &[(&'static str, Self)];

    #[inline]
    fn is_empty() -> bool {
        Self::VARIANT_MAPPINGS.is_empty()
    }
}

impl Format for () {
    const VARIANT_MAPPINGS: &[(&'static str, Self)] = &[];
}
