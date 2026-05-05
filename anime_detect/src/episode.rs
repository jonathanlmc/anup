use std::hash::Hash;

use crate::{Error, Format, Result, parse};

#[derive(Debug)]
pub struct Episode<F: Format = ()> {
    pub number: u32,
    pub season: Option<u16>,
    pub format: Option<F>,
}

impl<F: Format> Episode<F> {
    pub fn parse_with_known_filename(filename: &str) -> Result<Self> {
        if filename.is_empty() {
            return Err(Error::Unmatched);
        }

        let parsed = parse::episode::parse_complete_input(&mut &*filename);

        let format = parsed
            .as_ref()
            .ok()
            .and_then(|p| p.format)
            .or_else(|| parse::episode::isolated_series_format_label(&mut &*filename).ok());

        match (parsed, format) {
            (Ok(parsed), format) => Ok(Self {
                season: parsed.season,
                format,
                number: parsed.number,
            }),
            // if we at least have a type hint, treat this as a one-off
            (Err(_), format @ Some(_)) => Ok(Self {
                season: None,
                format,
                number: 1,
            }),
            (Err(_), _) => Err(Error::Unmatched),
        }
    }
}

impl<F> PartialEq for Episode<F>
where
    F: Format + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.season == other.season && self.number == other.number && self.format == other.format
    }
}

impl Eq for Episode {}

impl<F> Hash for Episode<F>
where
    F: Format + Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.number.hash(state);
        self.season.hash(state);
        self.format.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parse_filename {
        use std::fmt::Debug;

        use super::*;

        trait DebugFormat: Format + PartialEq + Debug {}

        impl<T> DebugFormat for T where T: Format + PartialEq + Debug {}

        #[derive(Copy, Clone, Debug, PartialEq)]
        enum TestFormat {
            TV,
            Movie,
        }

        impl Format for TestFormat {
            const VARIANT_MAPPINGS: &[(&'static str, Self)] =
                &[("tv", Self::TV), ("movie", Self::Movie)];
        }

        #[track_caller]
        fn cmp<F: DebugFormat>(input: &str, expected: Episode<F>) {
            assert_eq!(
                Episode::parse_with_known_filename(&mut &*input).ok(),
                Some(expected)
            );
        }

        #[track_caller]
        fn cmp_episode<F: DebugFormat>(input: &str, expected: u32) {
            cmp::<F>(
                input,
                Episode {
                    number: expected,
                    season: None,
                    format: None,
                },
            );
        }

        #[track_caller]
        fn cmp_ep_and_season<F: DebugFormat>(input: &str, expected_ep: u32, expected_season: u16) {
            cmp::<F>(
                input,
                Episode {
                    number: expected_ep,
                    season: Some(expected_season),
                    format: None,
                },
            );
        }

        #[track_caller]
        fn cmp_ep_and_type<F: DebugFormat>(
            input: &str,
            expected_ep: u32,
            expected_type: impl Into<Option<F>>,
        ) {
            cmp::<F>(
                input,
                Episode {
                    number: expected_ep,
                    season: None,
                    format: expected_type.into(),
                },
            );
        }

        #[track_caller]
        fn cmp_one_off<F: DebugFormat>(
            input: &str,
            season: Option<u16>,
            format_hint: impl Into<Option<F>>,
        ) {
            cmp::<F>(
                input,
                Episode {
                    number: 1,
                    format: format_hint.into(),
                    season,
                },
            )
        }

        #[test]
        fn with_episode_marker_only_succeeds() {
            for cmp_episode_impl in [cmp_episode::<()>, cmp_episode::<TestFormat>] {
                cmp_episode_impl("Series Title - 12", 12);
                cmp_episode_impl("Series Title - E12", 12);
                cmp_episode_impl("Series Title.-.E12", 12);
                cmp_episode_impl("E12 - Series Title", 12);
                cmp_episode_impl("Episode 12 - Series Title", 12);
                cmp_episode_impl("12 - Series Title", 12);
                cmp_episode_impl("12v2 - Series Title", 12);
                cmp_episode_impl("E12v2 - Series Title", 12);
                cmp_episode_impl("Series Title - E12v2", 12);
                cmp_episode_impl("Series Title - 12v2", 12);
                cmp_episode_impl("[Tag 1] 12 - Series Title", 12);
                cmp_episode_impl("[Tag 1] 12 - 1 Series Title", 12);
                cmp_episode_impl("[Tag 1] Episode 12 - Series Title", 12);
                cmp_episode_impl("[Tag 1] Episode 12 - Series Title 02", 12);
                cmp_episode_impl("[Tag 1] Ep12 - 1 Series Title", 12);
                cmp_episode_impl("[Tag_1]_Series_Title_-_12", 12);
                cmp_episode_impl("[Tag 1] Series Title - 12", 12);
                cmp_episode_impl("[Tag 1][Tag 2] Series Title - 12", 12);
                cmp_episode_impl("[Tag 1] [Tag 2] Series Title - 12", 12);
                cmp_episode_impl("Series Title 2 12", 12);
                cmp_episode_impl("Series Title 2 E12", 12);
                cmp_episode_impl("Series Title 2 Ep 12", 12);
                cmp_episode_impl("Series Title 2 12_(Tag 1)", 12);
                cmp_episode_impl("Series Title - 001", 1);
                cmp_episode_impl("Series Title - E001", 1);
                cmp_episode_impl("Series Title - ep 001", 1);
                cmp_episode_impl("Series Title - episode 001", 1);
                cmp_episode_impl("Series_Title_12", 12);
                cmp_episode_impl("Series Title-12", 12);
                cmp_episode_impl("Series Title 12 (1080p)", 12);
                cmp_episode_impl("Series Title 2 12 - An Episode Description [1080p]", 12);
                cmp_episode_impl("[Tag 1] Series 2 Title - 12 [Tag 2]", 12);
                cmp_episode_impl("[Tag 1] 1 2 Series Title - 06 [Tag 2]", 6);
                cmp_episode_impl("[Tag 1] 1 2 Series Title 06 [Tag 2]", 6);
                cmp_episode_impl("[Tag 1] 1 2 Series Title 3 06 [Tag 2]", 6);
                cmp_episode_impl("[Tag 1] 1 2 Series Title 3 - 06 [Tag 2]", 6);
                cmp_episode_impl("[Tag 1] Mutli-Separated 1-Title 2 - E12 [10]", 12);
                cmp_episode_impl("[Tag 1] 12 - Multi - Title [10]", 12);
                cmp_episode_impl("06", 6);
                cmp_episode_impl("07 [Tag 1]", 7);
                cmp_episode_impl("[Tag 1] 08", 8);
                cmp_episode_impl(
                    "[Tag 1] Episode 12 - Series Title - Description 123 [Tag 2]",
                    12,
                );
            }
        }

        #[test]
        fn with_season_hint_and_episode_succeeds() {
            for cmp_ep_and_season_impl in [cmp_ep_and_season::<()>, cmp_ep_and_season::<TestFormat>]
            {
                cmp_ep_and_season_impl("Series Title - S01E02", 2, 1);
                cmp_ep_and_season_impl("Series Title - S01E02 [Tag 1]", 2, 1);
                cmp_ep_and_season_impl("Series Title - S01E02v3", 2, 1);
                cmp_ep_and_season_impl("S01E12 - Series Title", 12, 1);

                // test that `Episode 1` isn't parsed as the episode
                cmp_ep_and_season_impl("S01E12 - Series Title - Episode 1 Description", 12, 1);

                cmp_ep_and_season_impl("[Tag] Series Title S2E01 (Tag 2)", 1, 2);
                cmp_ep_and_season_impl("[Tag] Series Title S02E01 (Tag 2)", 1, 2);
                cmp_ep_and_season_impl("[Tag] Series Title Season 2_-_01_(Tag 2)", 1, 2);
                cmp_ep_and_season_impl("[Tag] Series Title Season 2 Episode 01 (Tag 2)", 1, 2);
                cmp_ep_and_season_impl("[Tag] Series Title Season 02 Episode 01 (Tag 2)", 1, 2);
                cmp_ep_and_season_impl("[Tag] Series Title Season 2_-_01_(Tag 2)", 1, 2);
                cmp_ep_and_season_impl("[Tag] Series Title Season 2 - 01", 1, 2);
                cmp_ep_and_season_impl("[Tag] Series Title Season 2-01 (Tag 2)", 1, 2);
                cmp_ep_and_season_impl("[Tag] Series Title Season 2 Episode 1", 1, 2);
            }
        }

        #[test]
        fn with_format_hint_succeeds() {
            cmp_ep_and_type("Series Title TV - 12", 12, TestFormat::TV);
            cmp_ep_and_type("Series Title - 12 (TV)", 12, TestFormat::TV);
            cmp_ep_and_type("Series Title Movie - 12", 12, TestFormat::Movie);

            cmp(
                "Series Title - S01TV02",
                Episode {
                    number: 2,
                    season: Some(1),
                    format: Some(TestFormat::TV),
                },
            );

            cmp(
                "Series Title Movie - S00E01",
                Episode {
                    number: 1,
                    season: Some(0),
                    format: Some(TestFormat::Movie),
                },
            );

            cmp(
                "Series Title TVv50 - S01E06",
                Episode {
                    number: 6,
                    season: Some(1),
                    format: Some(TestFormat::TV),
                },
            );

            cmp_one_off("Series Title - TV", None, TestFormat::TV);
            cmp_one_off("Series Title - MovieV2", None, TestFormat::Movie);
        }

        #[test]
        #[should_panic]
        fn unknown_formats_are_not_parsed() {
            cmp_ep_and_type::<()>("Series Title ONA - 12", 12, None);
            // expected to fail since there is no episode number and no known format
            cmp_one_off::<()>("Series Title OVA", None, None);
        }
    }
}
