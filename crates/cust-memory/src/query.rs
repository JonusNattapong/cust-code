//! Query expansion for memory search.
//!
//! Removes stop words from conversational queries to extract meaningful keywords.
//! Pipeline: query → lowercase → split on non-alphanumeric → remove stop words → dedup → keywords.
//!
//! Returns empty vec when all words are stop words (caller should fallback to full-text).

use std::collections::HashSet;
use std::sync::LazyLock;

static STOP_WORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "a", "an", "the", "this", "that", "these", "those",
        "i", "me", "my", "we", "our", "you", "your", "he", "she", "it",
        "they", "him", "her", "its", "them", "us",
        "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did",
        "will", "would", "could", "should", "can", "may", "might",
        "in", "on", "at", "to", "for", "of", "with", "by", "from",
        "about", "into", "through", "during", "before", "after", "above", "below",
        "and", "or", "but", "if", "then", "because", "as", "while", "when",
        "where", "what", "which", "who", "how", "why",
        "thing", "things", "stuff", "something", "anything", "everything",
        "one", "some", "any", "all", "each", "every", "both", "few", "more",
        "yesterday", "today", "tomorrow", "earlier", "later", "recently", "now",
        "just", "already", "still", "yet",
        "please", "help", "find", "show", "get", "tell", "give", "make",
        "not", "no", "yes", "also", "too", "very", "really", "here",
        "there", "so", "up", "out", "like", "than", "other", "only",
    ]
    .into_iter()
    .collect()
});

/// Extract meaningful keywords from a query by removing stop words.
///
/// Returns keywords in order of appearance, deduplicated.
/// Filters words shorter than 2 chars and pure-numeric tokens.
pub fn extract_keywords(query: &str) -> Vec<String> {
    let lowered = query.to_lowercase();
    let mut seen = HashSet::new();
    lowered
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| w.len() >= 2)
        .filter(|w| !STOP_WORDS.contains(w))
        .filter(|w| !w.chars().all(|c| c.is_numeric()))
        .filter(|w| seen.insert(*w))
        .map(|w| w.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_stop_words() {
        let kw = extract_keywords("that thing we discussed about the API");
        assert_eq!(kw, vec!["discussed", "api"]);
    }

    #[test]
    fn all_stop_words_returns_empty() {
        let kw = extract_keywords("what is that?");
        assert!(kw.is_empty());
    }

    #[test]
    fn preserves_meaningful_words() {
        let kw = extract_keywords("rust programming async patterns");
        assert_eq!(kw, vec!["rust", "programming", "async", "patterns"]);
    }

    #[test]
    fn deduplicates() {
        let kw = extract_keywords("rust rust rust programming");
        assert_eq!(kw, vec!["rust", "programming"]);
    }

    #[test]
    fn filters_pure_numbers() {
        let kw = extract_keywords("port 8080 and 443 config");
        assert_eq!(kw, vec!["port", "config"]);
    }

    #[test]
    fn empty_query() {
        assert!(extract_keywords("").is_empty());
    }
}
