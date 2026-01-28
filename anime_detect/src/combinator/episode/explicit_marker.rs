//! Look for episode numbers in filenames that explicitly mark where the episode number is.

use winnow::{
    Parser, Result,
    ascii::Caseless,
    combinator::{alt, opt, repeat_till},
    token::any,
};

use crate::combinator::{
    episode::{ParsedEpAndSeason, parsed_digits, rest_of_filename, separator},
    none_or_many_tags, whitespace,
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
        season_followed_by_episode.map(|(season, episode)| ParsedEpAndSeason {
            number: episode,
            season_hint: Some(season),
        }),
        episode_marker.map(|episode| ParsedEpAndSeason {
            number: episode,
            season_hint: None,
        }),
    ));

    (marker, rest_of_filename)
        .map(|(marker, _)| marker)
        .parse_next(input)
}

fn season_followed_by_episode(input: &mut &str) -> Result<(u32, u32)> {
    winnow::seq!(
        season_marker,
        _: opt(whitespace),
        _: opt('-'),
        _: opt(whitespace),
        _: opt(episode_header),
        parsed_digits,
    )
    .map(|(season, episode)| (season, episode))
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

fn episode_header(input: &mut &str) -> Result<()> {
    alt((
        (Caseless("episode"), whitespace).void(),
        (Caseless("ep"), opt(whitespace)).void(),
        Caseless("e").void(),
    ))
    .parse_next(input)
}

pub fn episode_marker(input: &mut &str) -> Result<u32> {
    (episode_header, opt(separator), parsed_digits)
        .map(|(_, _, num)| num)
        .parse_next(input)
}
