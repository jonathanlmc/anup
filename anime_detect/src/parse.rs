pub mod episode;
pub mod series;

use winnow::{
    Parser, Result,
    ascii::Caseless,
    combinator::{alt, delimited, repeat, trace},
    error::{ContextError, ParserError},
    stream::{Offset, Stream},
    token::{one_of, take_until},
};

pub const RESOLUTION_TAGS: [Caseless<&str>; 5] = [
    Caseless("480p"),
    Caseless("720p"),
    Caseless("1080p"),
    Caseless("2160p"),
    Caseless("4320p"),
];

pub const WHITESPACE_CHARS: [char; 3] = [' ', '_', '.'];

pub fn any_tag<'a>(input: &mut &'a str) -> Result<&'a str> {
    trace(
        "any_tag",
        alt((parens_tag, brackets_tag, alt(RESOLUTION_TAGS))),
    )
    .parse_next(input)
}

pub fn any_tag_with_whitespace<'a>(input: &mut &'a str) -> Result<&'a str> {
    trace(
        "any_tag_with_whitespace",
        (maybe_whitespace, any_tag, maybe_whitespace).map(|(_, tag, _)| tag),
    )
    .parse_next(input)
}

pub fn many_tags_with_whitespace(input: &mut &str) -> Result<()> {
    trace(
        "many_tags_with_whitespace",
        repeat(0.., any_tag_with_whitespace),
    )
    .parse_next(input)
}

pub fn parens_tag<'a>(input: &mut &'a str) -> Result<&'a str> {
    trace("parens_tag", delimited('(', take_until(0.., ')'), ')')).parse_next(input)
}

pub fn brackets_tag<'a>(input: &mut &'a str) -> Result<&'a str> {
    trace("brackets_tag", delimited('[', take_until(0.., ']'), ']')).parse_next(input)
}

pub fn any_whitespace(input: &mut &str) -> Result<()> {
    trace("any_whitespace", move |input: &mut &str| {
        // faster than `take_while` and `repeat`
        let pos = input
            .find(|ch| !WHITESPACE_CHARS.contains(&ch))
            .filter(|p| *p > 0)
            .ok_or_else(|| ContextError::from_input(input))?;

        *input = &input[pos..];

        Ok(())
    })
    .parse_next(input)
}

pub fn maybe_whitespace(input: &mut &str) -> Result<()> {
    // for reasons unknown, `repeat` is faster than a hand-rolled loop (based off
    // naive benchmarks) *only* for this specific case where 0 occurrences are allowed
    trace(
        "maybe_whitespace",
        repeat(0.., one_of(WHITESPACE_CHARS)).map(|()| ()),
    )
    .parse_next(input)
}

/// Skip input one token at a time until the given parser succeeds.
/// Returns a tuple of (skipped input, parser result).
fn skip_till<I, O, E>(mut parser: impl Parser<I, O, E>) -> impl Parser<I, (I::Slice, O), E>
where
    I: Stream,
    E: winnow::error::ParserError<I>,
{
    trace("skip_till", move |input: &mut I| {
        let initial_pos = input.checkpoint();

        loop {
            let current_pos = input.checkpoint();

            match parser.parse_next(input) {
                Ok(res) => {
                    let end = input.checkpoint();

                    let offset = current_pos.offset_from(&initial_pos);

                    input.reset(&initial_pos);
                    let input_up_to_parser = input.next_slice(offset);
                    input.reset(&end);

                    return Ok((input_up_to_parser, res));
                }
                Err(err) if err.is_backtrack() => {
                    input.reset(&current_pos);
                    input.next_token().ok_or(ParserError::from_input(input))?;
                }
                Err(err) => return Err(err),
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    mod skip_till {
        use winnow::{error::ContextError, token::literal};

        use super::*;

        #[test]
        fn simple_parser_succeeds() {
            let result = skip_till::<_, _, ContextError>(winnow::ascii::digit1)
                .parse_next(&mut "abc123def")
                .unwrap();

            assert_eq!(result.0, "abc");
            assert_eq!(result.1, "123");
        }

        #[test]
        fn nested_parser_succeeds() {
            // Test skipping text with spaces
            let result = skip_till(any_tag)
                .parse_next(&mut "test input [tag content] with tag")
                .unwrap();

            assert_eq!(result.0, "test input ");
            assert_eq!(result.1, "tag content");
        }

        #[test]
        fn immediate_match_succeeds() {
            let result = skip_till::<_, _, ContextError>(literal("abc"))
                .parse_next(&mut "abc")
                .unwrap();

            assert_eq!(result.0, "");
            assert_eq!(result.1, "abc");
        }

        #[test]
        fn no_match_fails() {
            let result = skip_till::<_, _, ContextError>(winnow::ascii::digit1)
                .parse_next(&mut "no numbers here");

            assert!(result.is_err());
        }

        #[test]
        fn empty_input_fails() {
            let result = skip_till::<_, _, ContextError>(winnow::ascii::digit1).parse_next(&mut "");
            assert!(result.is_err());
        }
    }
}
