//! Look for an episode number within a filename.
//!
//! This module supports filenames containing either an explicitly or implicitly
//! marked episode number.
//!
//! For example, here are some filenames using implicit episode numbers:
//!
//! * `Series Title 01`
//! * `Series Title - 01`
//!
//! And some with explicit episode numbers:
//!
//! * `Series Title - Episode 01`
//! * `Series Title S01E01`

mod explicit_marker;
mod implicit_marker;

use std::str::FromStr;

use winnow::{
    Parser, Result,
    ascii::{Caseless, digit1},
    combinator::{alt, eof, fail, opt, trace},
    error::{ContextError, ParserError},
    token::literal,
};

use crate::{
    Format,
    parse::{any_tag, any_whitespace, many_tags_with_whitespace, maybe_whitespace, skip_till},
};

#[cfg_attr(test, derive(Debug, Default, PartialEq, Eq))]
pub struct Parsed<F: Format> {
    pub number: u32,
    pub season: Option<u16>,
    pub format: Option<F>,
}

pub fn parse_complete_input<F: Format>(filename: &mut &str) -> Result<Parsed<F>> {
    trace(
        "parse_complete_input",
        alt((
            // look for an explicit marker first before falling back to an implicit one
            // to help avoid false positive matches
            explicit_marker::parse_complete_input,
            implicit_marker::parse_complete_input,
        )),
    )
    .parse_next(filename)
}

pub fn separator(input: &mut &str) -> Result<()> {
    trace(
        "separator",
        (maybe_whitespace, '-', maybe_whitespace).void(),
    )
    .parse_next(input)
}

pub fn any_bare_series_format<F: Format>(input: &mut &str) -> Result<F> {
    trace("any_bare_series_format", move |input: &mut &str| {
        if F::is_empty() {
            return Err(ContextError::from_input(input));
        }

        for (name, value) in F::VARIANT_MAPPINGS {
            if literal::<_, _, ()>(Caseless(*name))
                .parse_next(input)
                .is_ok()
            {
                return Ok(*value);
            }
        }

        Err(ContextError::from_input(input))
    })
    .parse_next(input)
}

pub fn complete_series_format_label<F: Format>(input: &mut &str) -> Result<F> {
    const NAME: &str = "complete_series_format_label";

    if F::is_empty() {
        return trace(NAME, fail).parse_next(input);
    }

    trace(
        NAME,
        (
            alt((any_series_format_in_tag, any_bare_series_format)),
            opt(file_version),
        )
            .map(|(series_type, _)| series_type),
    )
    .parse_next(input)
}

pub fn any_series_format_in_tag<F: Format>(input: &mut &str) -> Result<F> {
    const NAME: &str = "any_series_format_in_tag";

    if F::is_empty() {
        return trace(NAME, fail).parse_next(input);
    }

    trace(NAME, any_tag.and_then(any_bare_series_format)).parse_next(input)
}

pub fn isolated_series_format_label<F: Format>(input: &mut &str) -> Result<F> {
    const NAME: &str = "isolated_series_format_label";

    if F::is_empty() {
        return trace(NAME, fail).parse_next(input);
    }

    trace(
        NAME,
        skip_till((
            alt((separator, any_whitespace)),
            complete_series_format_label,
            alt((separator, rest_of_filename_tags_only)),
        ))
        .map(|(_, (_, fmt, _))| fmt),
    )
    .parse_next(input)
}

pub fn parsed_digits<T: FromStr>(input: &mut &str) -> Result<T> {
    trace("parsed_digits", digit1.verify_map(|s: &str| s.parse().ok())).parse_next(input)
}

pub fn file_version(input: &mut &str) -> Result<()> {
    trace("file_version", (Caseless("v"), digit1).void()).parse_next(input)
}

pub fn rest_of_filename_tags_only(input: &mut &str) -> Result<()> {
    trace(
        "rest_of_filename_tags_only",
        (
            opt((separator, any_whitespace)),
            many_tags_with_whitespace,
            maybe_whitespace,
            eof,
        )
            .void(),
    )
    .parse_next(input)
}
