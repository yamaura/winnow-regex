pub trait Regex {
    type Haystack<'h>;

    fn capture_locations(&self) -> regex::CaptureLocations;
    fn captures_read(
        &self,
        locs: &mut regex::CaptureLocations,
        haystack: Self::Haystack<'_>,
    ) -> Option<(usize, usize)>;
}

impl Regex for regex::Regex {
    type Haystack<'h> = &'h str;

    fn capture_locations(&self) -> regex::CaptureLocations {
        regex::Regex::capture_locations(self)
    }

    fn captures_read(
        &self,
        locs: &mut regex::CaptureLocations,
        haystack: Self::Haystack<'_>,
    ) -> Option<(usize, usize)> {
        regex::Regex::captures_read(self, locs, haystack).map(|c| (c.start(), c.end()))
    }
}
