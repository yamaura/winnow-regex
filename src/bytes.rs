use crate::{CapturesParser, Error, Regex, RegexParser};
use core::fmt::Debug;
use winnow::{
    error::ParserError,
    stream::{Offset, Stream, StreamIsPartial},
};

pub use crate::Captures;

pub trait BytesRegexPattern {
    type Error;
    type Output;

    fn try_into_regex(self) -> Result<Self::Output, Self::Error>;

    /// Converts the pattern into a regex, panicking if it fails.
    /// ## Panics
    ///
    /// Panics if the regex pattern fails to compile.
    fn into_regex(self) -> Self::Output
    where
        Self: Sized,
        Self::Error: Debug,
    {
        self.try_into_regex()
            .unwrap_or_else(|e| panic!("failed to compile regex for bytes parser: {:?}", e))
    }
}

impl BytesRegexPattern for &str {
    type Error = Error;
    type Output = regex::bytes::Regex;

    #[inline(always)]
    fn try_into_regex(self) -> Result<Self::Output, Self::Error> {
        Ok(Self::Output::new(self)?)
    }
}

/// A `&[u8]`-oriented version of [`winnow_regex::regex`].
///
/// This parser matches the beginning of a byte stream (`&[u8]`)
/// using a regular expression compiled with [`regex::bytes::Regex`].
/// It returns the matching slice if the regex matches at offset 0.
///
/// For more usage details, see [`winnow_regex::regex`].
///
/// # Panics
///
/// Panics if the regex pattern fails to compile.
///
/// # Example
///
/// ```
/// use winnow::prelude::*;
/// use winnow_regex::bytes::regex;
///
/// fn digits<'i>(input: &mut &'i [u8]) -> ModalResult<&'i [u8]> {
///     regex(r"^\d+").parse_next(input)
/// }
///
/// assert_eq!(digits.parse_peek(b"123abc"), Ok((&b"abc"[..], &b"123"[..])));
/// ```
#[inline(always)]
pub fn regex<'h, Input, Re, Error>(re: Re) -> RegexParser<'h, Input, Re::Output, Error>
where
    Input: StreamIsPartial + Stream + Offset + Clone,
    Re: BytesRegexPattern,
    Re::Output: Regex<Haystack<'h> = <Input as Stream>::Slice>,
    Re::Error: Debug,
    Error: ParserError<Input> + 'static,
{
    let re = re.into_regex();

    RegexParser {
        re,
        _marker: core::marker::PhantomData,
    }
}

/// A `&[u8]`-oriented version of [`winnow_regex::captures`].
///
/// This parser matches and extracts capture groups from the beginning of a byte stream (`&[u8]`)
/// using a regular expression compiled with [`regex::bytes::Regex`].
/// If the regex matches at offset 0, all capture groups are returned.
///
/// For full semantics and error behavior, see [`winnow_regex::captures`].
///
/// # Panics
///
/// Panics if the regex pattern fails to compile.
///
/// # Example
///
/// ```
/// use winnow::prelude::*;
/// use winnow_regex::bytes::{captures, Captures};
///
/// fn coords(input: &mut &[u8]) -> ModalResult<(u32, u32)> {
///     captures(r"^(\d+),(\d+)")
///         .map(|c| {
///             let x = std::str::from_utf8(&c[1]).unwrap().parse().unwrap();
///             let y = std::str::from_utf8(&c[2]).unwrap().parse().unwrap();
///             (x, y)
///         })
///         .parse_next(input)
/// }
///
/// assert_eq!(coords.parse_peek(b"42,99 done"), Ok((&b" done"[..], (42, 99))));
/// ```
#[inline(always)]
pub fn captures<'h, Input, Re, Error>(re: Re) -> CapturesParser<'h, Input, Re::Output, Error>
where
    Input: StreamIsPartial + Stream + Offset + Clone,
    Re: BytesRegexPattern,
    Re::Output: Regex<Haystack<'h> = <Input as Stream>::Slice>,
    Re::Error: Debug,
    Error: ParserError<Input> + 'static,
{
    let re = re.into_regex();

    CapturesParser {
        re,
        _marker: core::marker::PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winnow::error::{ContextError, ErrMode, Needed};
    use winnow::prelude::*;

    #[test]
    fn partial() {
        use winnow::stream::Partial;
        fn partial<'i>(i: &mut Partial<&'i [u8]>) -> ModalResult<&'i [u8], ContextError> {
            regex(r"^\d+").parse_next(i)
        }
        assert_eq!(
            partial.parse_peek(Partial::new(&b"123abc"[..])),
            Ok((Partial::new(&b"abc"[..]), &b"123"[..]))
        );
        assert_eq!(
            partial.parse_peek(Partial::new(&b"123"[..])),
            Err(ErrMode::Incomplete(Needed::Unknown))
        );
    }

    #[test]
    fn test_re() {
        let re = regex::Regex::new(r"\d+").unwrap();
        assert!(re.find_at("1abc123", 0).is_some());
        assert!(re.find_at("1abc123", 1).is_some());
        assert!(re.find("abc123").is_some());
    }
}
