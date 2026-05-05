use winnow::{
    Parser, Result,
    ascii::digit1,
    combinator::{alt, eof, repeat, trace},
};

use crate::parse::{any_tag_with_whitespace, any_whitespace, skip_till};

pub fn trim_name(mut input: &str) -> Result<&str> {
    let many_tags = trace(
        "many_tags",
        repeat(0.., any_tag_with_whitespace).map(|()| ()).void(),
    );

    let title_until_tag_or_end = trace(
        "title_until_tag_or_end",
        skip_till(alt((
            any_tag_with_whitespace.void(),
            season_label.void(),
            eof.void(),
        )))
        .map(|(consumed, _)| consumed),
    );

    trace("trim_name", (many_tags, title_until_tag_or_end))
        .parse_next(&mut input)
        .map(|(_, name)| name)
}

// todo: support ranges (ex. `S01-04` and `S01-S04`)
fn season_label(input: &mut &str) -> Result<()> {
    trace(
        "season_label",
        (
            any_whitespace,
            'S',
            digit1,
            alt((any_whitespace, eof.void())),
        )
            .void(),
    )
    .parse_next(input)
}
