/// see
/// [`regex::CaptureLocations`](https://docs.rs/regex/latest/regex/struct.CaptureLocations.html)
/// for more details
#[allow(clippy::len_without_is_empty)]
pub trait CaptureLocations {
    type Input: ?Sized;
    fn get(&self, i: usize) -> Option<(usize, usize)>;
    fn len(&self) -> usize;
}

impl CaptureLocations for regex::CaptureLocations {
    type Input = str;
    #[inline]
    fn get(&self, i: usize) -> Option<(usize, usize)> {
        regex::CaptureLocations::get(self, i)
    }

    #[inline]
    fn len(&self) -> usize {
        regex::CaptureLocations::len(self)
    }
}

impl CaptureLocations for regex::bytes::CaptureLocations {
    type Input = [u8];
    #[inline]
    fn get(&self, i: usize) -> Option<(usize, usize)> {
        regex::bytes::CaptureLocations::get(self, i)
    }

    #[inline]
    fn len(&self) -> usize {
        regex::bytes::CaptureLocations::len(self)
    }
}

pub trait Regex {
    type Haystack<'h>;
    type CaptureLocations: CaptureLocations;

    fn capture_locations(&self) -> Self::CaptureLocations;
    fn captures_read(
        &self,
        locs: &mut Self::CaptureLocations,
        haystack: Self::Haystack<'_>,
    ) -> Option<(usize, usize)>;
}

impl Regex for regex::Regex {
    type Haystack<'h> = &'h str;
    type CaptureLocations = regex::CaptureLocations;

    #[inline]
    fn capture_locations(&self) -> Self::CaptureLocations {
        regex::Regex::capture_locations(self)
    }

    #[inline]
    fn captures_read(
        &self,
        locs: &mut Self::CaptureLocations,
        haystack: Self::Haystack<'_>,
    ) -> Option<(usize, usize)> {
        regex::Regex::captures_read(self, locs, haystack).map(|c| (c.start(), c.end()))
    }
}

impl Regex for regex::bytes::Regex {
    type Haystack<'h> = &'h [u8];
    type CaptureLocations = regex::bytes::CaptureLocations;

    #[inline]
    fn capture_locations(&self) -> Self::CaptureLocations {
        regex::bytes::Regex::capture_locations(self)
    }

    #[inline]
    fn captures_read(
        &self,
        locs: &mut Self::CaptureLocations,
        haystack: Self::Haystack<'_>,
    ) -> Option<(usize, usize)> {
        regex::bytes::Regex::captures_read(self, locs, haystack).map(|c| (c.start(), c.end()))
    }
}
