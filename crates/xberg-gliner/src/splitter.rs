use regex::Regex;

use crate::{Result, Token};

pub(crate) const DEFAULT_SPLITTER_REGEX: &str = r"\w+(?:[-_]\w+)*|\S";

pub(crate) trait Splitter {
    fn split(&self, input: &str, limit: Option<usize>) -> Result<Vec<Token>>;
}

pub(crate) struct RegexSplitter {
    regex: Regex,
}

impl RegexSplitter {
    pub(crate) fn new(regex: &str) -> Result<Self> {
        Ok(Self {
            regex: Regex::new(regex)?,
        })
    }
}

impl Splitter for RegexSplitter {
    fn split(&self, input: &str, limit: Option<usize>) -> Result<Vec<Token>> {
        let mut result = Vec::new();
        for match_ in self.regex.find_iter(input) {
            result.push(Token::new(match_.start(), match_.end(), match_.as_str()));
            if limit.is_some_and(|limit| result.len() >= limit) {
                break;
            }
        }
        Ok(result)
    }
}
