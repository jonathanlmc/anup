//! Look for episode numbers in filenames that explicitly mark where the episode number is.

use winnow::{
    Parser, Result,
    ascii::Caseless,
    combinator::{alt, opt, repeat_till},
    token::any,
};

use crate::{
    combinator::{
        episode::{ParsedEpAndSeason, parsed_digits, rest_of_filename, separator, type_hint_str},
        none_or_many_tags, whitespace,
    },
    series,
};

pub fn parse_filename(input: &mut &str) -> Result<ParsedEpAndSeason> {
    (
        // skip any tags to avoid false positives
        none_or_many_tags,
        repeat_till(0.., any, any_marker).map(|((), marker)| marker),
    )
        .map(|(_, marker)| marker)
        .parse_next(input)
}

fn any_marker(input: &mut &str) -> Result<ParsedEpAndSeason> {
    let marker = alt((
        season_followed_by_episode.map(|(season, format_hint, episode)| ParsedEpAndSeason {
            number: episode,
            season_hint: Some(season),
            format_hint,
        }),
        episode_marker.map(|(episode, format_hint)| ParsedEpAndSeason {
            number: episode,
            season_hint: None,
            format_hint,
        }),
    ));

    (marker, rest_of_filename)
        .map(|(marker, _)| marker)
        .parse_next(input)
}

fn season_followed_by_episode(input: &mut &str) -> Result<(u32, Option<series::Format>, u32)> {
    winnow::seq!(
        season_marker,
        _: opt(whitespace),
        _: opt('-'),
        _: opt(whitespace),
        opt(episode_header),
        parsed_digits,
    )
    .map(|(season, format_hint, episode)| (season, format_hint.flatten(), episode))
    .parse_next(input)
}

fn season_marker(input: &mut &str) -> Result<u32> {
    (
        alt((
            (Caseless("season"), whitespace).void(),
            Caseless("s").void(),
        )),
        parsed_digits,
    )
        .map(|(_, num)| num)
        .parse_next(input)
}

fn episode_header(input: &mut &str) -> Result<Option<series::Format>> {
    alt((
        (Caseless("episode"), whitespace).map(|_| None),
        (Caseless("ep"), opt(whitespace)).map(|_| None),
        Caseless("e").map(|_| None),
        type_hint_str.map(Some),
    ))
    .parse_next(input)
}

pub fn episode_marker(input: &mut &str) -> Result<(u32, Option<series::Format>)> {
    (episode_header, opt(separator), parsed_digits)
        .map(|(format, _, num)| (num, format))
        .parse_next(input)
}
