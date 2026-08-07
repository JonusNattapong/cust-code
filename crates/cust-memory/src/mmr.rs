//! Maximal Marginal Relevance (MMR) diversity re-ranking.
//!
//! Without MMR, top memory results about the same topic are nearly identical.
//! MMR penalizes redundancy by balancing relevance with diversity.
//!
//! Formula: MMR(d) = λ × relevance(d) - (1-λ) × max_similarity(d, selected)
//!
//! Uses Jaccard similarity on tokenized snippets (no embeddings needed).
//! O(n²) but n is tiny (typically 6–18 candidates).

use std::collections::HashSet;

/// A search result from memory retrieval.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    pub score: f64,
    pub source: String,
}

fn tokenize(text: &str) -> HashSet<&str> {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| !w.is_empty())
        .collect()
}

fn jaccard_similarity(a: &HashSet<&str>, b: &HashSet<&str>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count();
    let union = a.len() + b.len() - intersection;
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Re-rank results using Maximal Marginal Relevance.
///
/// Reorders `results` in-place. No-op when `lambda >= 1.0` or fewer than 2 results.
pub fn mmr_rerank(results: &mut Vec<SearchResult>, relevance: &[f64], lambda: f64) {
    if results.len() <= 1 || lambda >= 1.0 {
        return;
    }
    assert_eq!(relevance.len(), results.len());

    let lowered: Vec<String> = results.iter().map(|r| r.content.to_lowercase()).collect();
    let token_cache: Vec<HashSet<&str>> = lowered.iter().map(|s| tokenize(s)).collect();

    let max_score = relevance.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min_score = relevance.iter().copied().fold(f64::INFINITY, f64::min);
    let range = (max_score - min_score).max(f64::EPSILON);

    let mut selected: Vec<usize> = Vec::with_capacity(results.len());
    let mut remaining: Vec<usize> = (0..results.len()).collect();

    while !remaining.is_empty() {
        let mut best_pos = 0;
        let mut best_mmr = f64::NEG_INFINITY;

        for (pos, &candidate) in remaining.iter().enumerate() {
            let normalized = (relevance[candidate] - min_score) / range;
            let max_sim = selected
                .iter()
                .map(|&sel| jaccard_similarity(&token_cache[candidate], &token_cache[sel]))
                .fold(0.0_f64, f64::max);

            let mmr_score = lambda * normalized - (1.0 - lambda) * max_sim;

            if mmr_score > best_mmr
                || (mmr_score == best_mmr
                    && relevance[candidate] > relevance[remaining[best_pos]])
            {
                best_mmr = mmr_score;
                best_pos = pos;
            }
        }

        selected.push(remaining.remove(best_pos));
    }

    *results = selected
        .into_iter()
        .map(|i| std::mem::replace(&mut results[i], SearchResult {
            id: String::new(),
            content: String::new(),
            score: 0.0,
            source: String::new(),
        }))
        .collect();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(id: &str, content: &str, score: f64) -> SearchResult {
        SearchResult {
            id: id.to_string(),
            content: content.to_string(),
            score,
            source: "test".to_string(),
        }
    }

    #[test]
    fn single_result_noop() {
        let mut results = vec![make_result("a", "rust async", 1.0)];
        let relevance = [1.0];
        mmr_rerank(&mut results, &relevance, 0.5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "a");
    }

    #[test]
    fn lambda_one_noop() {
        let mut results = vec![
            make_result("a", "rust async", 1.0),
            make_result("b", "python sync", 0.5),
        ];
        let relevance = [1.0, 0.5];
        mmr_rerank(&mut results, &relevance, 1.0);
        assert_eq!(results[0].id, "a");
        assert_eq!(results[1].id, "b");
    }

    #[test]
    fn diverse_results_promoted() {
        let mut results = vec![
            make_result("a", "rust async programming patterns", 1.0),
            make_result("b", "rust async programming tutorial", 0.95),
            make_result("c", "python web framework flask", 0.9),
        ];
        let relevance: Vec<f64> = results.iter().map(|r| r.score).collect();
        mmr_rerank(&mut results, &relevance, 0.5);

        assert_eq!(results[0].id, "a");
        assert_eq!(
            results[1].id, "c",
            "diverse result should be promoted over redundant one"
        );
    }

    #[test]
    fn identical_snippets_penalized() {
        let mut results = vec![
            make_result("a", "exact same content here", 1.0),
            make_result("b", "exact same content here", 0.99),
            make_result("c", "completely different topic", 0.5),
        ];
        let relevance: Vec<f64> = results.iter().map(|r| r.score).collect();
        mmr_rerank(&mut results, &relevance, 0.5);

        assert_eq!(results[0].id, "a");
        assert_eq!(
            results[1].id, "c",
            "different result should beat identical duplicate"
        );
    }

    #[test]
    fn case_insensitive_similarity() {
        let mut results = vec![
            make_result("a", "Rust Async Programming", 1.0),
            make_result("b", "rust async programming", 0.95),
            make_result("c", "Python Web Framework", 0.9),
        ];
        let relevance: Vec<f64> = results.iter().map(|r| r.score).collect();
        mmr_rerank(&mut results, &relevance, 0.5);

        assert_eq!(results[0].id, "a");
        assert_eq!(results[1].id, "c", "case-only difference should be detected");
    }

    #[test]
    fn jaccard_identical() {
        let a: HashSet<&str> = ["rust", "async"].into();
        let b: HashSet<&str> = ["rust", "async"].into();
        assert!((jaccard_similarity(&a, &b) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn jaccard_disjoint() {
        let a: HashSet<&str> = ["rust", "async"].into();
        let b: HashSet<&str> = ["python", "web"].into();
        assert!(jaccard_similarity(&a, &b).abs() < f64::EPSILON);
    }
}
