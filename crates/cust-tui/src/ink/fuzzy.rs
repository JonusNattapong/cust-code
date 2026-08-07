//! Fuzzy subsequence matching and ranking.
//!
//! Port of `pi-tui`'s `src/fuzzy.ts`. A query matches when all its characters
//! appear in the text in order, not necessarily adjacent. **Lower score is a
//! better match** — the scoring is a penalty budget, not a similarity measure.

/// Outcome of matching a query against one candidate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FuzzyMatch {
    pub matches: bool,
    /// Lower is better. Meaningless when `matches` is false.
    pub score: f64,
}

const NO_MATCH: FuzzyMatch = FuzzyMatch {
    matches: false,
    score: 0.0,
};

fn is_word_boundary(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '-' | '_' | '.' | '/' | ':')
}

fn match_query(query: &[char], text: &[char]) -> FuzzyMatch {
    if query.is_empty() {
        return FuzzyMatch {
            matches: true,
            score: 0.0,
        };
    }
    if query.len() > text.len() {
        return NO_MATCH;
    }

    let mut qi = 0usize;
    let mut score = 0.0f64;
    let mut last_match: Option<usize> = None;
    let mut consecutive = 0usize;

    for (i, &c) in text.iter().enumerate() {
        if qi >= query.len() {
            break;
        }
        if c != query[qi] {
            continue;
        }
        let at_boundary = i == 0 || is_word_boundary(text[i - 1]);

        if last_match == Some(i.wrapping_sub(1)) && i > 0 {
            // Runs of adjacent characters are what users actually type.
            consecutive += 1;
            score -= consecutive as f64 * 5.0;
        } else {
            consecutive = 0;
            if let Some(last) = last_match {
                score += (i - last - 1) as f64 * 2.0;
            }
        }
        if at_boundary {
            score -= 10.0;
        }
        // Matches further into the string are marginally worse.
        score += i as f64 * 0.1;

        last_match = Some(i);
        qi += 1;
    }

    if qi < query.len() {
        return NO_MATCH;
    }
    if query == text {
        score -= 100.0;
    }
    FuzzyMatch {
        matches: true,
        score,
    }
}

/// Split a query into a leading letter run and trailing digit run, or the
/// reverse, and return it with the two halves swapped.
///
/// Covers the common transposition where someone types `f2` for `2f`.
fn swapped_query(q: &[char]) -> Option<Vec<char>> {
    let letters_first = q.iter().position(|c| c.is_ascii_digit())?;
    if letters_first == 0 {
        // digits then letters
        let split = q.iter().position(|c| c.is_ascii_alphabetic())?;
        if !q[..split].iter().all(char::is_ascii_digit) || !q[split..].iter().all(char::is_ascii_alphabetic) {
            return None;
        }
        let mut out = q[split..].to_vec();
        out.extend_from_slice(&q[..split]);
        return Some(out);
    }
    if !q[..letters_first].iter().all(char::is_ascii_alphabetic)
        || !q[letters_first..].iter().all(char::is_ascii_digit)
    {
        return None;
    }
    let mut out = q[letters_first..].to_vec();
    out.extend_from_slice(&q[..letters_first]);
    Some(out)
}

/// Match `query` against `text`, case-insensitively.
pub fn fuzzy_match(query: &str, text: &str) -> FuzzyMatch {
    let q: Vec<char> = query.to_lowercase().chars().collect();
    let t: Vec<char> = text.to_lowercase().chars().collect();

    let primary = match_query(&q, &t);
    if primary.matches {
        return primary;
    }
    // Fall back to the letter/digit transposition before giving up.
    match swapped_query(&q) {
        Some(swapped) => {
            let alt = match_query(&swapped, &t);
            if alt.matches {
                alt
            } else {
                primary
            }
        }
        None => primary,
    }
}

/// A candidate with its match score attached.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredItem<T> {
    pub item: T,
    pub score: f64,
}

/// Filter and rank `items` by fuzzy match on the string `key` extracts.
///
/// Results are sorted best-first (ascending score); ties keep input order.
pub fn fuzzy_filter_scored<T, F>(items: Vec<T>, query: &str, key: F) -> Vec<ScoredItem<T>>
where
    F: Fn(&T) -> String,
{
    let mut scored: Vec<ScoredItem<T>> = items
        .into_iter()
        .filter_map(|item| {
            let m = fuzzy_match(query, &key(&item));
            m.matches.then_some(ScoredItem { item, score: m.score })
        })
        .collect();
    // Stable sort so equal scores preserve the caller's ordering.
    scored.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

/// Filter and rank, discarding the scores.
pub fn fuzzy_filter<T, F>(items: Vec<T>, query: &str, key: F) -> Vec<T>
where
    F: Fn(&T) -> String,
{
    fuzzy_filter_scored(items, query, key)
        .into_iter()
        .map(|s| s.item)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_matches_everything() {
        assert!(fuzzy_match("", "anything").matches);
    }

    #[test]
    fn matches_out_of_order_characters_only_in_sequence() {
        assert!(fuzzy_match("abc", "aXbXc").matches);
        assert!(!fuzzy_match("cba", "abc").matches);
    }

    #[test]
    fn query_longer_than_text_cannot_match() {
        assert!(!fuzzy_match("abcdef", "abc").matches);
    }

    #[test]
    fn matching_is_case_insensitive() {
        assert!(fuzzy_match("ABC", "abc").matches);
    }

    #[test]
    fn exact_match_scores_best() {
        let exact = fuzzy_match("commit", "commit");
        let partial = fuzzy_match("commit", "commit-all");
        assert!(exact.score < partial.score);
    }

    #[test]
    fn consecutive_beats_scattered() {
        let dense = fuzzy_match("abc", "abcxyz");
        let sparse = fuzzy_match("abc", "axbxcx");
        assert!(dense.score < sparse.score);
    }

    #[test]
    fn word_boundary_matches_rank_higher() {
        let boundary = fuzzy_match("gs", "git status");
        let inner = fuzzy_match("gs", "gists");
        assert!(boundary.score < inner.score);
    }

    #[test]
    fn falls_back_to_letter_digit_transposition() {
        // "f2" fails literally against "2fa" but "2f" succeeds.
        assert!(!match_query(&['f', '2'], &['2', 'f', 'a']).matches);
        assert!(fuzzy_match("f2", "2fa").matches);
    }

    #[test]
    fn transposition_does_not_rescue_a_genuine_miss() {
        assert!(!fuzzy_match("z9", "abc").matches);
    }

    #[test]
    fn filter_sorts_best_first_and_drops_misses() {
        let items = vec!["unrelated", "commit-all", "commit"];
        let got = fuzzy_filter(items, "commit", |s| s.to_string());
        assert_eq!(got, vec!["commit", "commit-all"]);
    }

    #[test]
    fn filter_with_empty_query_keeps_input_order() {
        let items = vec!["b", "a", "c"];
        assert_eq!(fuzzy_filter(items, "", |s| s.to_string()), vec!["b", "a", "c"]);
    }
}
