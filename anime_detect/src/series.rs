use crate::{Error, Result, parse};

#[derive(Debug, PartialEq, Eq)]
pub struct Series<'a> {
    pub trimmed_name: &'a str,
}

impl<'a> Series<'a> {
    pub fn parse(value: &'a str) -> Result<Self> {
        if value.is_empty() {
            return Err(Error::Unmatched);
        }

        parse::series::trim_name(value)
            .map(|parsed| Self {
                trimmed_name: parsed,
            })
            .map_err(|_| Error::Unmatched)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parse_name {
        use super::*;

        #[track_caller]
        fn cmp(untrimmed: &str, expected: &str) {
            assert_eq!(
                Series::parse(untrimmed).ok().map(|s| s.trimmed_name),
                Some(expected.into())
            );
        }

        #[test]
        fn no_tags() {
            cmp("Title", "Title");
            cmp("Series Title", "Series Title");

            assert!(Series::parse("").is_err());
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
