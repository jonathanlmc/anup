//! Look for episode numbers in filenames that **do not** explicitly mark where the episode number is.

use winnow::{
    Parser, Result,
    ascii::digit1,
    combinator::{alt, not, opt, repeat_till},
    token::any,
};

use crate::combinator::{
    episode::{
        ParsedEpAndSeason, file_version, parsed_digits, rest_of_filename,
        rest_of_filename_tags_only, separator,
    },
    none_or_many_tags, whitespace,
};

pub fn parse_filename(input: &mut &str) -> Result<ParsedEpAndSeason> {
    let middle_of_file_episode =
        repeat_till(1.., any, episode_in_middle_of_file).map(|((), episode)| episode);

    (
        // skip tags to avoid false positives
        none_or_many_tags,
        alt((
            middle_of_file_episode,
            episode_number_only,
            episode_number_at_start,
        )),
    )
        .map(|(_, episode)| ParsedEpAndSeason {
            number: episode,
            season_hint: None,
            format_hint: None,
        })
        .parse_next(input)
}

fn episode_in_middle_of_file(input: &mut &str) -> Result<u32> {
    let skip_description_until_end = (
        separator,
        // edge case: if there's digits right after the separator, this is likely the
        // real episode number, and not the digits that were already parsed with
        // `episode_number`, so fail the parser so these digits can be tried during
        // a later iteration
        //
        // example title with this case: `1 Series Title 2 - 06`
        not(digit1),
        repeat_till(1.., any, rest_of_filename_tags_only).map(|((), _)| ()),
    )
        .void();

    (
        episode_number,
        alt((rest_of_filename_tags_only, skip_description_until_end)),
    )
        .map(|(episode, _)| episode)
        .parse_next(input)
}

fn episode_number_only(input: &mut &str) -> Result<u32> {
    (parsed_digits, rest_of_filename_tags_only)
        .map(|(episode, _)| episode)
        .parse_next(input)
}

fn episode_number_at_start(input: &mut &str) -> Result<u32> {
    (
        parsed_digits,
        // version string
        opt((opt(whitespace), file_version)),
        // require the separator, since this can be too ambiguous with
        // a series title otherwise
        separator,
        rest_of_filename,
    )
        .map(|(episode, _, _, _)| episode)
        .parse_next(input)
}

pub fn episode_number(input: &mut &str) -> Result<u32> {
    (
        alt((whitespace, separator)),
        parsed_digits,
        // if there's more digits that follow closely after this set,
        // then the parsed digits above are likely a season number and not
        // the episode number we're actually looking for, so fail
        // the parser in that case
        not((alt((whitespace, separator)), digit1)),
    )
        .map(|(_, episode, _)| episode)
        .parse_next(input)
}
