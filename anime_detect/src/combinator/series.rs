use crate::combinator::{tag, whitespace};
use winnow::{
    Parser, Result,
    ascii::digit1,
    combinator::{alt, eof, repeat, repeat_till},
    token::any,
};

pub fn trim_name(mut name: &str) -> Result<String> {
    let many_tags = repeat(0.., tag).map(|()| ()).void();

    let title_until_tag_or_end = repeat_till(1.., any, alt((tag, season_label.take(), eof)))
        .map(|(str, _)| -> String { str });

    (many_tags, title_until_tag_or_end)
        .parse_next(&mut name)
        .map(|(_, name)| name)
}

fn season_label(input: &mut &str) -> Result<()> {
    (whitespace, 'S', digit1, alt((whitespace, eof.void())))
        .void()
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod trim_name {
        use super::*;

        #[track_caller]
        fn cmp(untrimmed: &str, expected: &str) {
            assert_eq!(trim_name(untrimmed), Ok(expected.into()));
        }

        #[test]
        fn no_tags() {
            cmp("Title", "Title");
            cmp("Series Title", "Series Title");

            assert!(trim_name("").is_err());
        }

        #[test]
        fn one_tag() {
            cmp("[Tag] Title", "Title");
            cmp("[Tag] Series Title", "Series Title");
            cmp("Series Title [Tag]", "Series Title");
            cmp("(Tag) Series Title", "Series Title");
            cmp("Series Title (Tag)", "Series Title");
            cmp("Series Title 720p", "Series Title");
            cmp("Series Title 1080p", "Series Title");
            cmp("Series Title S1", "Series Title");
            cmp("Series Title S01", "Series Title");
            cmp("Series Title S12", "Series Title");
        }

        #[test]
        fn many_tags() {
            cmp("[Tag1] [Tag2] Series Title", "Series Title");
            cmp("[Tag1] (Tag2) Series Title", "Series Title");
            cmp("Series Title [Tag1] [Tag2]", "Series Title");
            cmp("[Tag1] Series Title 1080p", "Series Title");
            cmp("[Tag1] Series Title 1080p (Tag 2) (Tag 3)", "Series Title");
            cmp("[Tag1] Series Title S01 [Tag2] (Tag3)", "Series Title")
        }
    }
}
