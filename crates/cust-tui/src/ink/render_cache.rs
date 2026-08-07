//! Width+version keyed render memoization.
//!
//! Port of `pi-tui`'s `src/render-cache.ts`. A component bumps a version
//! counter when its content changes; the cache hits only when both the version
//! and the render width match, so a resize invalidates naturally.

#[derive(Debug, Default, Clone)]
pub struct VersionedRenderCache {
    key: Option<(usize, u64)>,
    lines: Vec<String>,
}

impl VersionedRenderCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, width: usize, version: u64) -> Option<&[String]> {
        match self.key {
            Some(k) if k == (width, version) => Some(&self.lines),
            _ => None,
        }
    }

    pub fn set(&mut self, width: usize, version: u64, lines: Vec<String>) -> &[String] {
        self.key = Some((width, version));
        self.lines = lines;
        &self.lines
    }

    pub fn invalidate(&mut self) {
        self.key = None;
        self.lines.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hits_only_on_matching_width_and_version() {
        let mut c = VersionedRenderCache::new();
        c.set(80, 1, vec!["a".into()]);
        assert_eq!(c.get(80, 1), Some(&["a".to_string()][..]));
        assert!(c.get(80, 2).is_none());
        assert!(c.get(100, 1).is_none());
    }

    #[test]
    fn invalidate_clears() {
        let mut c = VersionedRenderCache::new();
        c.set(80, 1, vec!["a".into()]);
        c.invalidate();
        assert!(c.get(80, 1).is_none());
    }
}
