pub mod series;

use winnow::{
    Parser, Result,
    ascii::Caseless,
    combinator::{alt, delimited, opt, repeat},
    token::take_until,
};

pub const RESOLUTION_TAGS: [Caseless<&str>; 5] = [
    Caseless("480p"),
    Caseless("720p"),
    Caseless("1080p"),
    Caseless("2160p"),
    Caseless("4320p"),
];

pub fn tag<'a>(input: &mut &'a str) -> Result<&'a str> {
    (
        opt(whitespace),
        alt((brackets, parens, alt(RESOLUTION_TAGS))),
        opt(whitespace),
    )
        .map(|(_, tag, _)| tag)
        .parse_next(input)
}

pub fn parens<'a>(input: &mut &'a str) -> Result<&'a str> {
    delimited('(', take_until(0.., ')'), ')').parse_next(input)
}

pub fn brackets<'a>(input: &mut &'a str) -> Result<&'a str> {
    delimited('[', take_until(0.., ']'), ']').parse_next(input)
}

pub fn whitespace(input: &mut &str) -> Result<()> {
    repeat(1.., alt((' ', '_', '.')))
        .map(|()| ())
        .parse_next(input)
}
