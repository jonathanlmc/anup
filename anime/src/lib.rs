#![warn(
    clippy::pedantic,
    clippy::nursery,
    clippy::dbg_macro,
    clippy::mod_module_files,
    clippy::shadow_unrelated,
    clippy::if_then_some_else_none,
    clippy::redundant_type_annotations,
    clippy::mutex_atomic,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::redundant_test_prefix,
    clippy::unnecessary_self_imports,
    clippy::unneeded_field_pattern
)]
#![allow(clippy::missing_errors_doc)]

use serde::Deserialize;

pub mod api;

pub type PlainId = u32;

#[derive(Copy, Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum Id {
    AniList(PlainId),
}

impl Id {
    #[inline]
    #[must_use]
    pub const fn source_name(self) -> &'static str {
        match self {
            Self::AniList(_) => "AniList",
        }
    }

    #[inline]
    #[must_use]
    pub const fn plain(self) -> PlainId {
        match self {
            Self::AniList(id) => id,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Info {
    pub id: Id,
    pub title: Title,
    pub cover_image_url: CoverImage,
    pub episodes: Option<u32>,
    pub format: Option<Format>,
    pub user_list_entry: Option<UserListEntry>,
}

#[derive(Clone, Debug)]
pub struct UserListEntry {
    pub completed_at: Option<jiff::civil::Date>,
    pub created_at: Option<jiff::Timestamp>,
    pub progress: Option<u32>,
    pub repeat: Option<u32>,
    pub score: Option<f32>,
    pub started_at: Option<jiff::civil::Date>,
    pub status: Option<UserStatus>,
    pub updated_at: Option<jiff::Timestamp>,
}

#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum UserStatus {
    Current,
    Planning,
    Completed,
    Dropped,
    Paused,
    Repeating,
}

impl UserStatus {
    #[inline]
    #[must_use]
    pub const fn display_str(self) -> &'static str {
        match self {
            Self::Current => "Watching",
            Self::Planning => "Planning",
            Self::Completed => "Completed",
            Self::Dropped => "Dropped",
            Self::Paused => "Paused",
            Self::Repeating => "Rewatching",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Title {
    pub english: Option<String>,
    pub romaji: String,
    pub native: String,
}

impl Title {
    #[inline]
    #[must_use]
    pub fn as_array(&self) -> [Option<&str>; 3] {
        [
            self.english.as_deref(),
            Some(&self.romaji),
            Some(&self.native),
        ]
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverImage {
    /// URL pointing to the extra large cover image for the series.
    pub extra_large: Option<String>,
}

#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum Format {
    Tv,
    Movie,
    Special,
    Ona,
    Ova,
    Music,
    Other,
}

mod macros {
    macro_rules! include_str_from_root {
        ($relative_path:expr) => {
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $relative_path))
        };
    }

    macro_rules! include_graphql {
        ($relative_path:expr) => {
            $crate::macros::include_str_from_root!(concat!("generated/graphql/", $relative_path))
        };
    }

    pub(crate) use include_graphql;
    pub(crate) use include_str_from_root;
}
