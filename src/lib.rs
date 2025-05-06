#![doc = include_str!("../README.md")]
pub use winnow;

pub mod regex_trait;

use core::fmt::Debug;
use regex_trait::*;
use winnow::{
    Parser,
    error::{Needed, ParserError},
    stream::{Offset, Stream, StreamIsPartial},
};

#[derive(Debug, Clone, thiserror::Error)]
#[error(transparent)]
pub enum Error {
    Regex(#[from] regex::Error),
}

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
pub trait RegexPattern {
    type Error;
    type Output;

    fn try_into_regex(self) -> Result<Self::Output, Self::Error>;
}

impl RegexPattern for &str {
    type Error = Error;
    type Output = regex::Regex;

    #[inline(always)]
    fn try_into_regex(self) -> Result<Self::Output, Self::Error> {
        Ok(Self::Output::new(self)?)
    }
}

impl RegexPattern for String {
    type Error = Error;
    type Output = regex::Regex;

    #[inline(always)]
    fn try_into_regex(self) -> Result<Self::Output, Self::Error> {
        Ok(Self::Output::new(&self)?)
    }
}

impl RegexPattern for regex::Regex {
    type Error = Error;
    type Output = regex::Regex;

    #[inline(always)]
    fn try_into_regex(self) -> Result<Self::Output, Self::Error> {
        Ok(self)
    }
}

impl RegexPattern for regex::bytes::Regex {
    type Error = Error;
    type Output = regex::bytes::Regex;

    #[inline(always)]
    fn try_into_regex(self) -> Result<Self::Output, Self::Error> {
        Ok(self)
    }
}

pub struct Captures<Slice, L>
where
    L: CaptureLocations,
{
    slice: Slice,
    locs: L,
}

impl<Slice, L, T: ?Sized> AsRef<T> for Captures<Slice, L>
where
    Slice: AsRef<T>,
    L: CaptureLocations,
{
    fn as_ref(&self) -> &T {
        self.slice.as_ref()
    }
}

impl<Slice, L> core::ops::Index<usize> for Captures<Slice, L>
where
    Slice: AsRef<str>,
    L: CaptureLocations,
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

pub struct RegexParser<'h, I, R, E>
where
    I: Stream + StreamIsPartial + Offset + Clone,
    R: Regex<Haystack<'h> = <I as Stream>::Slice>,
    E: ParserError<I>,
{
    re: R,
    _marker: core::marker::PhantomData<(&'h (), I, E)>,
}

impl<'h, I, R, E> Parser<I, <I as Stream>::Slice, E> for RegexParser<'h, I, R, E>
where
    I: Stream + StreamIsPartial + Offset + Clone,
    R: Regex<Haystack<'h> = <I as Stream>::Slice>,
    E: ParserError<I>,
{
    fn parse_next(&mut self, input: &mut I) -> Result<<I as Stream>::Slice, E> {
        if <I as StreamIsPartial>::is_partial_supported() {
            captures_impl::<_, _, _, true>(input, &self.re)
        } else {
            captures_impl::<_, _, _, false>(input, &self.re)
        }
        .map(|caps| caps.slice)
    }
}

pub struct CapturesParser<'h, I, R, E>
where
    I: Stream,
    R: Regex,
    E: ParserError<I>,
{
    re: R,
    _marker: core::marker::PhantomData<(&'h (), I, E)>,
}

impl<'h, I, R, E> Parser<I, Captures<<I as Stream>::Slice, R::CaptureLocations>, E>
    for CapturesParser<'h, I, R, E>
where
    I: Stream + StreamIsPartial + Offset + Clone,
    R: Regex<Haystack<'h> = <I as Stream>::Slice>,
    E: ParserError<I>,
{
    fn parse_next(
        &mut self,
        input: &mut I,
    ) -> Result<Captures<<I as Stream>::Slice, R::CaptureLocations>, E> {
        if <I as StreamIsPartial>::is_partial_supported() {
            captures_impl::<_, _, _, true>(input, &self.re)
        } else {
            captures_impl::<_, _, _, false>(input, &self.re)
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
//pub fn regex<Input, Re, Error>(re: Re) -> impl Parser<Input, <Input as Stream>::Slice, Error>
pub fn regex<'h, Input, Re, Error>(re: Re) -> RegexParser<'h, Input, Re::Output, Error>
where
    Input: StreamIsPartial + Stream + Offset + Clone,
    Re: RegexPattern,
    //Re::Output: Regex,
    Re::Output: Regex<Haystack<'h> = <Input as Stream>::Slice>,
    Re::Error: Debug,
    Error: ParserError<Input> + 'static,
{
    let re = re.try_into_regex().expect("regex compile error");

    RegexParser {
        re,
        _marker: core::marker::PhantomData,
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
pub fn captures<'h, Input, Re, Error>(re: Re) -> CapturesParser<'h, Input, Re::Output, Error>
where
    Input: StreamIsPartial + Stream + Offset,
    Re: RegexPattern,
    Re::Output: Regex,
    Re::Error: Debug,
    Error: ParserError<Input> + 'static,
{
    let re = re.try_into_regex().expect("regex compile error");

    CapturesParser {
        re,
        _marker: core::marker::PhantomData,
    }
}

fn captures_impl<'h, I, Re, E, const PARTIAL: bool>(
    input: &mut I,
    re: &Re,
) -> Result<Captures<<I as Stream>::Slice, Re::CaptureLocations>, E>
where
    I: Stream + StreamIsPartial + Offset + Clone,
    Re: Regex<Haystack<'h> = <I as Stream>::Slice>,
    E: ParserError<I>,
{
    let hay = input.peek_finish();
    let mut locs = re.capture_locations();

    match re.captures_read(&mut locs, hay) {
        Some((start, end)) if start == 0 => {
            let len = end;
            if PARTIAL && input.is_partial() && input.eof_offset() == end {
                Err(E::incomplete(input, Needed::Unknown))
            } else {
                Ok(Captures {
                    slice: input.next_slice(len),
                    locs,
                })
            }
        }
        _ if PARTIAL && input.is_partial() => Err(E::incomplete(input, Needed::Unknown)),
        _ => Err(ParserError::from_input(input)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winnow::error::{ContextError, EmptyError, ErrMode};
    use winnow::prelude::*;

    #[test]
    fn regex_parser() {
        let mut p: RegexParser<&str, regex::Regex, EmptyError> = RegexParser {
            re: regex::Regex::new(r"^\d+").unwrap(),
            _marker: core::marker::PhantomData,
        };
        assert_eq!(p.parse_peek("42abc"), Ok(("abc", "42")));
    }

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
    fn partial() {
        use winnow::stream::Partial;
        fn partial<'i>(i: &mut Partial<&'i [u8]>) -> ModalResult<&'i [u8], ContextError> {
            regex(regex::bytes::Regex::new(r"^\d+").unwrap()).parse_next(i)
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
