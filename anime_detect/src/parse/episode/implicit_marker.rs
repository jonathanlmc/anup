//! Look for episode numbers in filenames that **do not** explicitly mark where the episode number is.

use winnow::{
    Parser, Result,
    ascii::digit1,
    combinator::{alt, not, opt, trace},
};

use crate::{
    Format,
    parse::{
        any_whitespace,
        episode::{Parsed, file_version, parsed_digits, rest_of_filename_tags_only, separator},
        many_tags_with_whitespace, maybe_whitespace, skip_till,
    },
};

pub fn parse_complete_input<F: Format>(input: &mut &str) -> Result<Parsed<F>> {
    trace(
        "parse_complete_input",
        (
            // skip tags to avoid false positives
            many_tags_with_whitespace,
            alt((
                episode_in_middle_of_input,
                episode_number_only,
                episode_number_at_start,
            )),
        )
            .map(|(_, episode)| Parsed {
                number: episode,
                season: None,
                format: None,
            }),
    )
    .parse_next(input)
}

fn episode_in_middle_of_input(input: &mut &str) -> Result<u32> {
    let skip_description_until_end = trace(
        "skip_description_until_end",
        (
            separator,
            // edge case: if there's digits right after the separator, this is likely the
            // real episode number, and not the digits that were already parsed with
            // `episode_number`, so fail the parser so these digits can be tried during
            // a later iteration
            //
            // example title with this case: `1 Series Title 2 - 06`
            not(digit1),
            skip_till(rest_of_filename_tags_only).void(),
        )
            .void(),
    );

    trace(
        "episode_in_middle_of_file",
        skip_till((
            episode_number,
            alt((rest_of_filename_tags_only, skip_description_until_end)),
        ))
        .map(|(_, (episode, _))| episode),
    )
    .parse_next(input)
}

fn episode_number_only(input: &mut &str) -> Result<u32> {
    trace(
        "episode_number_only",
        (parsed_digits, rest_of_filename_tags_only).map(|(episode, _)| episode),
    )
    .parse_next(input)
}

fn episode_number_at_start(input: &mut &str) -> Result<u32> {
    trace(
        "episode_number_at_start",
        (
            parsed_digits,
            // version string
            opt((maybe_whitespace, file_version)),
            // require the separator, since this can be too ambiguous with
            // a series title otherwise
            separator,
        )
            .map(|(episode, _, _)| episode),
    )
    .parse_next(input)
}

pub fn episode_number(input: &mut &str) -> Result<u32> {
    trace(
        "episode_number",
        (
            alt((any_whitespace, separator)),
            parsed_digits,
            // if there's more digits that follow closely after this set,
            // then the parsed digits above are likely a season number and not
            // the episode number we're actually looking for, so fail
            // the parser in that case
            not((alt((any_whitespace, separator)), digit1)),
        )
            .map(|(_, episode, _)| episode),
    )
    .parse_next(input)
}
