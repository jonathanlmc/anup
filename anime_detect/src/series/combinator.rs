use winnow::{
    Parser, Result,
    ascii::Caseless,
    combinator::{alt, delimited, eof, opt, repeat, repeat_till},
    token::{any, take_until},
};

const RESOLUTION_TAGS: [Caseless<&str>; 5] = [
    Caseless("480p"),
    Caseless("720p"),
    Caseless("1080p"),
    Caseless("2160p"),
    Caseless("4320p"),
];

pub fn trim_name(mut name: &str) -> Result<String> {
    let many_tags = repeat(0.., tag).map(|()| ()).void();

    let title_until_tag_or_end =
        repeat_till(1.., any, alt((tag, eof))).map(|(chars, _)| -> Vec<char> { chars });

    (many_tags, title_until_tag_or_end)
        .parse_next(&mut name)
        .map(|(_, name)| name.into_iter().collect())
}

fn tag<'a>(input: &mut &'a str) -> Result<&'a str> {
    (
        opt(whitespace),
        alt((brackets, parens, alt(RESOLUTION_TAGS))),
        opt(whitespace),
    )
        .map(|(_, tag, _)| tag)
        .parse_next(input)
}

fn parens<'a>(input: &mut &'a str) -> Result<&'a str> {
    delimited('(', take_until(0.., ')'), ')').parse_next(input)
}

fn brackets<'a>(input: &mut &'a str) -> Result<&'a str> {
    delimited('[', take_until(0.., ']'), ']').parse_next(input)
}

fn whitespace(input: &mut &str) -> Result<()> {
    repeat(1.., alt((' ', '_', '.')))
        .map(|()| ())
        .parse_next(input)
}
