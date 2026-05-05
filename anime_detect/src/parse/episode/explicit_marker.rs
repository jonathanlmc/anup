//! Look for episode numbers in filenames that explicitly mark where the episode number is.

use winnow::{
    Parser, Result,
    ascii::Caseless,
    combinator::{alt, opt, trace},
};

use crate::{
    Format,
    parse::{
        any_whitespace,
        episode::{Parsed, any_bare_series_format, parsed_digits, separator},
        many_tags_with_whitespace, maybe_whitespace, skip_till,
    },
};

pub fn parse_complete_input<F: Format>(input: &mut &str) -> Result<Parsed<F>> {
    trace(
        "parse_complete_input",
        (
            // skip any tags to avoid false positives
            many_tags_with_whitespace,
            skip_till(any_marker_type).map(|(_, marker)| marker),
        )
            .map(|(_, marker)| marker),
    )
    .parse_next(input)
}

pub fn any_marker_type<F: Format>(input: &mut &str) -> Result<Parsed<F>> {
    trace(
        "any_marker_type",
        alt((
            season_followed_by_episode.map(|(season, format, episode)| Parsed {
                number: episode,
                season: Some(season),
                format,
            }),
            episode_num_marker.map(|(episode, format)| Parsed {
                number: episode,
                season: None,
                format,
            }),
        )),
    )
    .parse_next(input)
}

pub fn season_followed_by_episode<F: Format>(input: &mut &str) -> Result<(u16, Option<F>, u32)> {
    trace(
        "season_followed_by_episode",
        winnow::seq!(
            season_marker,
            _: maybe_whitespace,
            _: opt('-'),
            _: maybe_whitespace,
            opt(episode_header),
            parsed_digits,
        )
        .map(|(season, format_hint, episode)| (season, format_hint.flatten(), episode)),
    )
    .parse_next(input)
}

pub fn season_marker(input: &mut &str) -> Result<u16> {
    trace(
        "season_marker",
        (
            alt((
                (Caseless("season"), any_whitespace).void(),
                Caseless("s").void(),
            )),
            parsed_digits,
        )
            .map(|(_, num)| num),
    )
    .parse_next(input)
}

pub fn episode_header<F: Format>(input: &mut &str) -> Result<Option<F>> {
    trace(
        "episode_header",
        alt((
            (Caseless("episode"), any_whitespace).map(|_| None),
            (Caseless("ep"), maybe_whitespace).map(|_| None),
            Caseless("e").map(|_| None),
            any_bare_series_format.map(Some),
        )),
    )
    .parse_next(input)
}

pub fn episode_num_marker<F: Format>(input: &mut &str) -> Result<(u32, Option<F>)> {
    trace(
        "episode_num_marker",
        (episode_header, opt(separator), parsed_digits).map(|(format, _, num)| (num, format)),
    )
    .parse_next(input)
}
