//! Look for an episode number within a filename.
//!
//! This module supports filenames containing either an explicitly or implicitly
//! marked episode number.
//!
//! For example, here are some filenames using implicit episode numbers:
//!
//! * `Series Title 01.mkv`
//! * `Series Title - 01.mkv`
//!
//! And some with explicit episode numbers:
//!
//! * `Series Title - Episode 01.mkv`
//! * `Series Title S01E01.mkv`

mod explicit_marker;
mod implicit_marker;

use winnow::{
    Parser, Result,
    ascii::{Caseless, digit1},
    combinator::{alt, eof, opt, repeat, repeat_till},
    token::any,
};

use crate::{
    combinator::{any_tag_start, tag, whitespace},
    series,
};

const CONTAINER_EXTENSIONS: [Caseless<&str>; 17] = [
    Caseless("avi"),
    Caseless("flv"),
    Caseless("m2ts"),
    Caseless("mts"),
    Caseless("m4v"),
    Caseless("mkv"),
    Caseless("mov"),
    Caseless("mp4"),
    Caseless("mpg"),
    Caseless("mpeg"),
    Caseless("ogv"),
    Caseless("ts"),
    Caseless("webm"),
    Caseless("wmv"),
    Caseless("3gp"),
    Caseless("3g2"),
    Caseless("f4v"),
];

#[cfg_attr(test, derive(Debug, Default, PartialEq, Eq))]
pub struct Parsed {
    pub season_hint: Option<u32>,
    pub series_type_hint: series::Format,
    pub number: u32,
}

#[cfg_attr(test, derive(Debug, Default, PartialEq, Eq))]
struct ParsedEpAndSeason {
    number: u32,
    season_hint: Option<u32>,
    format_hint: Option<series::Format>,
}

pub fn parse_filename(mut filename: &str) -> Result<Parsed> {
    let episode_and_season = {
        let mut fname_copy = filename;

        alt((
            // look for an explicit marker first before falling back to an implicit one
            // to help avoid false positive matches
            explicit_marker::parse_filename,
            implicit_marker::parse_filename,
        ))
        .parse_next(&mut fname_copy)
    };

    let type_hint = episode_and_season
        .as_ref()
        .ok()
        .and_then(|p| p.format_hint)
        .or_else(|| {
            repeat_till(0.., any, type_hint_label)
                .map(|((), hint)| hint)
                .parse_next(&mut filename)
                .ok()
        });

    match (episode_and_season, type_hint) {
        (Ok(ep_and_season), type_hint) => Ok(Parsed {
            season_hint: ep_and_season.season_hint,
            // we'll only have a type hint if the filename has an explicit special-like marker
            // (such as `special`, `ona`, or `ova`), so not having like *probably* means
            // this is a TV series
            series_type_hint: type_hint.unwrap_or(series::Format::TV),
            number: ep_and_season.number,
        }),
        // if we at least have a type hint for a special-like episode, treat this as a one-off
        (Err(_), Some(type_hint)) if type_hint != series::Format::TV => Ok(Parsed {
            season_hint: Some(0),
            series_type_hint: type_hint,
            number: 1,
        }),
        (Err(err), _) => Err(err),
    }
}

fn separator(input: &mut &str) -> Result<()> {
    (opt(whitespace), '-', opt(whitespace))
        .void()
        .parse_next(input)
}

fn type_hint_label(input: &mut &str) -> Result<series::Format> {
    (
        alt((separator, whitespace)),
        // it's not really worth the complexity trying to make sure this
        // tag ends properly if there does happen to be one here
        opt(any_tag_start),
        type_hint_str,
        opt(file_version),
    )
        .map(|(_, _, series_type, _)| series_type)
        .parse_next(input)
}

fn type_hint_str(input: &mut &str) -> Result<series::Format> {
    alt((
        (Caseless("special"), opt(Caseless("s"))).map(|_| series::Format::Special),
        // case-sensitive to avoid false positives
        "ONA".map(|_| series::Format::ONA),
        "OVA".map(|_| series::Format::OVA),
        Caseless("movie").map(|_| series::Format::Movie),
    ))
    .parse_next(input)
}

fn parsed_digits(input: &mut &str) -> Result<u32> {
    digit1
        .verify_map(|s: &str| s.parse().ok())
        .parse_next(input)
}

fn file_version(input: &mut &str) -> Result<()> {
    (Caseless("v"), digit1).void().parse_next(input)
}

fn rest_of_filename_tags_only(input: &mut &str) -> Result<()> {
    (
        opt((separator, whitespace)),
        repeat(0.., tag).map(|()| ()),
        opt(whitespace),
        file_extension_and_eof,
    )
        .void()
        .parse_next(input)
}

fn rest_of_filename(input: &mut &str) -> Result<()> {
    (repeat_till(0.., any, file_extension_and_eof))
        .map(|((), _)| ())
        .parse_next(input)
}

fn file_extension_and_eof(input: &mut &str) -> Result<()> {
    (alt(CONTAINER_EXTENSIONS), eof).void().parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parse_filename {
        use super::*;

        #[track_caller]
        fn cmp(input: &str, expected: Parsed) {
            assert_eq!(parse_filename(input), Ok(expected));
        }

        #[track_caller]
        fn cmp_episode(input: &str, expected: u32) {
            cmp(
                input,
                Parsed {
                    number: expected,
                    ..Default::default()
                },
            );
        }

        #[track_caller]
        fn cmp_ep_and_season(input: &str, expected_ep: u32, expected_season: u32) {
            cmp(
                input,
                Parsed {
                    number: expected_ep,
                    season_hint: Some(expected_season),
                    ..Default::default()
                },
            );
        }

        #[track_caller]
        fn cmp_ep_and_type(input: &str, expected_ep: u32, expected_type: series::Format) {
            cmp(
                input,
                Parsed {
                    number: expected_ep,
                    series_type_hint: expected_type,
                    ..Default::default()
                },
            );
        }

        #[test]
        fn with_episode_marker_only() {
            cmp_episode("Series Title - 12.mkv", 12);
            cmp_episode("Series Title - E12.mkv", 12);
            cmp_episode("Series Title.-.E12.mkv", 12);
            cmp_episode("E12 - Series Title.mkv", 12);
            cmp_episode("Episode 12 - Series Title.mkv", 12);
            cmp_episode("12 - Series Title.mkv", 12);
            cmp_episode("12v2 - Series Title.mkv", 12);
            cmp_episode("E12v2 - Series Title.mkv", 12);
            cmp_episode("Series Title - E12v2.mkv", 12);
            cmp_episode("Series Title - 12v2.mkv", 12);
            cmp_episode("[Tag 1] 12 - Series Title.mkv", 12);
            cmp_episode("[Tag 1] 12 - 1 Series Title.mkv", 12);
            cmp_episode("[Tag 1] Episode 12 - Series Title.mkv", 12);
            cmp_episode("[Tag 1] Episode 12 - Series Title 02.mkv", 12);
            cmp_episode("[Tag 1] Ep12 - 1 Series Title.mkv", 12);
            cmp_episode("[Tag_1]_Series_Title_-_12.mkv", 12);
            cmp_episode("[Tag 1] Series Title - 12.mkv", 12);
            cmp_episode("[Tag 1][Tag 2] Series Title - 12.mkv", 12);
            cmp_episode("[Tag 1] [Tag 2] Series Title - 12.mkv", 12);
            cmp_episode("Series Title 2 12.mkv", 12);
            cmp_episode("Series Title 2 E12.mkv", 12);
            cmp_episode("Series Title 2 Ep 12.mkv", 12);
            cmp_episode("Series Title 2 12_(Tag 1).mkv", 12);
            cmp_episode("Series Title - 001.mkv", 1);
            cmp_episode("Series Title - E001.mkv", 1);
            cmp_episode("Series Title - ep 001.mkv", 1);
            cmp_episode("Series Title - episode 001.mkv", 1);
            cmp_episode("Series_Title_12.mkv", 12);
            cmp_episode("Series Title-12.mkv", 12);
            cmp_episode("Series Title 12 (1080p).mkv", 12);
            cmp_episode("Series Title 2 12 - An Episode Description [1080p].mkv", 12);
            cmp_episode("[Tag 1] Series 2 Title - 12 [Tag 2].mkv", 12);
            cmp_episode("[Tag 1] 1 2 Series Title - 06 [Tag 2].mkv", 6);
            cmp_episode("[Tag 1] 1 2 Series Title 06 [Tag 2].mkv", 6);
            cmp_episode("[Tag 1] 1 2 Series Title 3 06 [Tag 2].mkv", 6);
            cmp_episode("[Tag 1] 1 2 Series Title 3 - 06 [Tag 2].mkv", 6);
            cmp_episode("[Tag 1] Mutli-Separated 1-Title 2 - E12 [10].mkv", 12);
            cmp_episode("[Tag 1] 12 - Multi - Title [10].mkv", 12);
            cmp_episode("06.mkv", 6);
            cmp_episode("07 [Tag 1].mkv", 7);
            cmp_episode("[Tag 1] 08.mkv", 8);
            cmp_episode(
                "[Tag 1] Episode 12 - Series Title - Description 123 [Tag 2].mkv",
                12,
            );
        }

        #[test]
        fn with_season_hint_and_episode() {
            cmp_ep_and_season("Series Title - S01E02.mkv", 2, 1);
            cmp_ep_and_season("Series Title - S01E02 [Tag 1].mkv", 2, 1);
            cmp_ep_and_season("Series Title - S01E02v3.mkv", 2, 1);
            cmp_ep_and_season("S01E12 - Series Title.mkv", 12, 1);

            // test that `Episode 1` isn't parsed as the episode
            cmp_ep_and_season("S01E12 - Series Title - Episode 1 Description.mkv", 12, 1);

            cmp_ep_and_season("[Tag] Series Title S2E01 (Tag 2).mkv", 1, 2);
            cmp_ep_and_season("[Tag] Series Title S02E01 (Tag 2).mkv", 1, 2);
            cmp_ep_and_season("[Tag] Series Title Season 2_-_01_(Tag 2).mkv", 1, 2);
            cmp_ep_and_season("[Tag] Series Title Season 2 Episode 01 (Tag 2).mkv", 1, 2);
            cmp_ep_and_season("[Tag] Series Title Season 02 Episode 01 (Tag 2).mkv", 1, 2);
            cmp_ep_and_season("[Tag] Series Title Season 2_-_01_(Tag 2).mkv", 1, 2);
            cmp_ep_and_season("[Tag] Series Title Season 2 - 01.mkv", 1, 2);
            cmp_ep_and_season("[Tag] Series Title Season 2-01 (Tag 2).mkv", 1, 2);
            cmp_ep_and_season("[Tag] Series Title Season 2 Episode 1.mkv", 1, 2);
        }

        #[test]
        fn with_type_hint() {
            cmp_ep_and_type("Series Title ONA - 12.mkv", 12, series::Format::ONA);
            cmp_ep_and_type("Series Title - 12 (ONA).mkv", 12, series::Format::ONA);
            cmp_ep_and_type("Series Title OVA - 12.mkv", 12, series::Format::OVA);
            cmp_ep_and_type("Series Title Special - 12.mkv", 12, series::Format::Special);

            cmp(
                "Series Title - S01OVA02.mkv",
                Parsed {
                    number: 2,
                    season_hint: Some(1),
                    series_type_hint: series::Format::OVA,
                },
            );

            cmp_ep_and_type(
                "Series Title Specials - 12.mkv",
                12,
                series::Format::Special,
            );

            let cmp_one_off = |input, type_hint| {
                cmp(
                    input,
                    Parsed {
                        number: 1,
                        series_type_hint: type_hint,
                        season_hint: Some(0),
                        ..Default::default()
                    },
                )
            };

            cmp_one_off(
                "Series Title Specials - S00E01.mkv",
                series::Format::Special,
            );
            cmp_one_off("Series Title - ONA.mkv", series::Format::ONA);
            cmp_one_off("Series Title - ONAv2.mkv", series::Format::ONA);
            cmp_one_off("Series Title Movie.mkv", series::Format::Movie);
        }
    }
}
