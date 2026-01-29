mod combinator;

pub mod episode;
pub mod series;

pub use episode::{Episode, EpisodeSet};
pub use series::{ParseSeriesError, TopLevelSeries};
