#![doc = include_str!("../README.md")]
pub use winnow;

use core::fmt::Debug;
use regex::Regex;
use winnow::{
    Parser,
    error::{Needed, ParserError},
    stream::{Offset, Stream, StreamIsPartial},
};

/// A trait representing types that can be converted into a compiled [`Regex`] pattern.
///
/// This is used by the `regex` parser to generically accept either a `&str` or an already-compiled
/// [`Regex`] object. Implementors of this trait can be converted into a `Regex` via the
/// [`try_into_regex`] method, allowing flexible API usage.
///
/// # Associated Types
///
/// - `Error`: The error type returned if regex compilation fails.
///
/// # Required Methods
///
/// - `try_into_regex(self) -> Result<Regex, Self::Error>`: Attempts to compile or convert
///   the input into a [`Regex`] object.
#[derive(Debug, Clone, thiserror::Error)]
#[error(transparent)]
pub enum Error {
    Regex(#[from] regex::Error),
}

pub trait RegexPattern {
    type Error;

    fn try_into_regex(self) -> Result<Regex, Self::Error>;
}

impl RegexPattern for &str {
    type Error = Error;

    #[inline]
    fn try_into_regex(self) -> Result<Regex, Self::Error> {
        Ok(Regex::new(self)?)
    }
}

impl RegexPattern for String {
    type Error = Error;
    fn try_into_regex(self) -> Result<Regex, Self::Error> {
        Ok(Regex::new(&self)?)
    }
}

impl RegexPattern for Regex {
    type Error = Error;

    #[inline]
    fn try_into_regex(self) -> Result<Regex, Self::Error> {
        Ok(self)
    }
}

pub struct Match<'h, T: ?Sized> {
    hay: &'h T,
    start: usize,
    end: usize,
}

impl<'h> Match<'h, str> {
    pub fn as_str(&self) -> &'h str {
        &self.hay[self.start..self.end]
    }
}

pub struct Captures<Slice> {
    slice: Slice,
    locs: regex::CaptureLocations,
}

impl<Slice, T: ?Sized> AsRef<T> for Captures<Slice>
where
    Slice: AsRef<T>,
{
    fn as_ref(&self) -> &T {
        self.slice.as_ref()
    }
}

impl<Slice> core::ops::Index<usize> for Captures<Slice>
where
    Slice: AsRef<str>,
{
    type Output = str;

    fn index(&self, i: usize) -> &Self::Output {
        if let Some((start, end)) = self.locs.get(i) {
            &self.as_ref()[start..end]
        } else {
            panic!("index out of bounds")
        }
    }
}

/// Creates a parser that matches input using a regular expression.
///
/// This parser takes a regular expression pattern (implementing [`RegexPattern`])
/// and returns a parser that attempts to match from the **beginning** of the input.
/// If the regular expression does not match at position 0, the parser fails.
///
/// Internally, this uses a precompiled [`Regex`] from the [`regex`] crate and supports
/// both complete and partial input modes via the [`StreamIsPartial`] trait.
///
/// # Panics
///
/// Panics if the regex pattern fails to compile.
///
/// # Example
///
/// ```
/// use winnow::prelude::*;
/// use winnow_regex::regex;
///
/// fn digits<'i>(s: &mut &'i str) -> ModalResult<&'i str> {
///     regex(r"^\d+").parse_next(s)
/// }
///
/// assert_eq!(digits.parse_peek("42abc"), Ok(("abc", "42")));
/// assert!(digits.parse_peek("abc42").is_err());
///
/// // Example with precompiled regex
/// fn word<'i>(s: &mut &'i str) -> ModalResult<&'i str> {
///     let re = regex::Regex::new(r"^\w+").unwrap();
///     regex(re).parse_next(s)
/// }
///
/// assert_eq!(word.parse_peek("hello world"), Ok((" world", "hello")));
/// assert!(word.parse_peek("!hello").is_err());
/// ```
#[inline(always)]
pub fn regex<Input, Re, Error>(re: Re) -> impl Parser<Input, <Input as Stream>::Slice, Error>
where
    Input: StreamIsPartial + Stream + AsRef<str> + Offset,
    Re: RegexPattern + 'static,
    Re::Error: Debug,
    Error: ParserError<Input> + 'static,
{
    let re = re.try_into_regex().expect("regex compile error");

    move |i: &mut Input| {
        if <Input as StreamIsPartial>::is_partial_supported() {
            captures_impl::<_, _, true>(i, &re)
        } else {
            captures_impl::<_, _, false>(i, &re)
        }
        .map(|caps| caps.slice)
    }
}

/// # Example
/// ```
/// use winnow::prelude::*;
/// use winnow_regex::{captures, Captures};
///
/// fn digits<'i>(s: &mut &'i str) -> ModalResult<(i32, i32)> {
///    captures(r"^(\d+)x(\d+)").map(|c| (c[1].parse().unwrap(), c[2].parse().unwrap())).parse_next(s)
/// }
///
/// assert_eq!(digits.parse_peek("11x42abc"), Ok(("abc", (11, 42))));
/// ```
#[inline(always)]
pub fn captures<'m, Input, Re, Error>(
    re: Re,
) -> impl Parser<Input, Captures<<Input as Stream>::Slice>, Error>
where
    Input: StreamIsPartial + Stream + AsRef<str> + Offset,
    <Input as Stream>::Slice: AsRef<str>,
    Re: RegexPattern + 'static,
    Re::Error: Debug,
    Error: ParserError<Input> + 'static,
{
    let re = re.try_into_regex().expect("regex compile error");

    move |i: &mut Input| {
        if <Input as StreamIsPartial>::is_partial_supported() {
            captures_impl::<_, _, true>(i, &re)
        } else {
            captures_impl::<_, _, false>(i, &re)
        }
    }
}

fn captures_impl<I, E, const PARTIAL: bool>(
    input: &mut I,
    re: &Regex,
) -> Result<Captures<<I as Stream>::Slice>, E>
where
    I: Stream + StreamIsPartial + AsRef<str> + Offset,
    E: ParserError<I>,
{
    let hay: &str = input.as_ref();
    let mut locs = re.capture_locations();

    match re.captures_read(&mut locs, hay) {
        Some(m) if m.start() == 0 => {
            let len = m.end();
            //Ok((input.next_slice(len), locs))
            Ok(Captures {
                slice: input.next_slice(len),
                locs,
            })
        }
        _ if PARTIAL && input.is_partial() => Err(E::incomplete(input, Needed::Unknown)),
        _ => Err(ParserError::from_input(input)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winnow::error::EmptyError;
    use winnow::prelude::*;

    #[test]
    fn ok_with_literal_pattern() {
        fn digits<'i>(s: &mut &'i str) -> ModalResult<&'i str> {
            regex(r"^\d+").parse_next(s)
        }
        assert_eq!(digits.parse_peek("42xyz"), Ok(("xyz", "42")));
    }

    #[test]
    fn unicode_partial() {
        let mut s = "あいう123";
        let re = regex::<_, _, EmptyError>(r"^[ぁ-ん]+")
            .parse_next(&mut s)
            .unwrap();
        assert_eq!(re, "あいう");
    }

    #[test]
    fn test_re() {
        let re = Regex::new(r"\d+").unwrap();
        assert!(re.find_at("1abc123", 0).is_some());
        assert!(re.find_at("1abc123", 1).is_some());
        assert!(re.find("abc123").is_some());
    }
}
