//! Fuzzy matching algorithm for file search

/// Match score with detailed information
#[derive(Debug, Clone, PartialEq)]
pub struct MatchScore {
    /// Overall score (0.0 to 1.0, higher is better)
    pub score: f64,

    /// Matched character positions in the target string
    pub matched_positions: Vec<usize>,

    /// Type of match
    pub match_type: MatchType,
}

/// Types of matches with different priorities
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchType {
    /// Exact match (highest priority)
    Exact,

    /// Continuous substring match
    Continuous,

    /// Match at the beginning of the string
    Prefix,

    /// Match at the beginning of a word
    WordStart,

    /// Fuzzy match with gaps
    Fuzzy,
}

/// High-performance fuzzy matcher
pub struct FuzzyMatcher {
    /// Case sensitivity setting
    case_sensitive: bool,
}

impl FuzzyMatcher {
    /// Create a new fuzzy matcher
    pub fn new(case_sensitive: bool) -> Self {
        Self { case_sensitive }
    }

    /// Match a query against a target string
    pub fn match_string(&self, query: &str, target: &str) -> Option<MatchScore> {
        if query.is_empty() {
            return Some(MatchScore {
                score: 1.0,
                matched_positions: Vec::new(),
                match_type: MatchType::Exact,
            });
        }

        let query = if self.case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };
        let target = if self.case_sensitive {
            target.to_string()
        } else {
            target.to_lowercase()
        };

        // Try different match types in order of priority

        // 1. Exact match
        if query == target {
            return Some(MatchScore {
                score: 1.0,
                matched_positions: (0..target.len()).collect(),
                match_type: MatchType::Exact,
            });
        }

        // 2. Continuous substring match
        if let Some(start_pos) = target.find(&query) {
            let score = self.calculate_continuous_score(&query, &target, start_pos);
            return Some(MatchScore {
                score,
                matched_positions: (start_pos..start_pos + query.len()).collect(),
                match_type: if start_pos == 0 {
                    MatchType::Prefix
                } else {
                    MatchType::Continuous
                },
            });
        }

        // 3. Word start match
        if let Some((score, positions)) = self.try_word_start_match(&query, &target) {
            return Some(MatchScore {
                score,
                matched_positions: positions,
                match_type: MatchType::WordStart,
            });
        }

        // 4. Fuzzy match
        if let Some((score, positions)) = self.try_fuzzy_match(&query, &target) {
            return Some(MatchScore {
                score,
                matched_positions: positions,
                match_type: MatchType::Fuzzy,
            });
        }

        None
    }

    /// Calculate score for continuous matches
    fn calculate_continuous_score(&self, query: &str, target: &str, start_pos: usize) -> f64 {
        let base_score = query.len() as f64 / target.len() as f64;

        // Bonus for matches at the beginning
        let position_bonus = if start_pos == 0 { 0.3 } else { 0.1 };

        // Bonus for exact length match
        let length_bonus = if query.len() == target.len() {
            0.2
        } else {
            0.0
        };

        // Ensure minimum score for continuous matches
        let final_score = (base_score + position_bonus + length_bonus).min(1.0);
        final_score.max(0.8) // Ensure continuous matches get high scores
    }

    /// Try to match at word boundaries
    fn try_word_start_match(&self, query: &str, target: &str) -> Option<(f64, Vec<usize>)> {
        let query_chars: Vec<char> = query.chars().collect();
        let target_chars: Vec<char> = target.chars().collect();

        let mut matched_positions = Vec::new();
        let mut query_idx = 0;
        let mut consecutive_matches = 0;
        let mut total_matches = 0;

        for (target_idx, &target_char) in target_chars.iter().enumerate() {
            if query_idx >= query_chars.len() {
                break;
            }

            let is_word_start = target_idx == 0
                || target_chars[target_idx - 1] == '_'
                || target_chars[target_idx - 1] == '-'
                || target_chars[target_idx - 1] == '/'
                || target_chars[target_idx - 1] == '.'
                || (target_chars[target_idx - 1].is_lowercase() && target_char.is_uppercase());

            if target_char == query_chars[query_idx] {
                if is_word_start {
                    matched_positions.push(target_idx);
                    query_idx += 1;
                    consecutive_matches += 1;
                    total_matches += 1;
                } else {
                    // Character matches but not at word start - this disqualifies word start match
                    return None;
                }
            }
        }

        if query_idx == query_chars.len() {
            let score = (total_matches as f64 / target_chars.len() as f64) * 0.8
                + (consecutive_matches as f64 / query_chars.len() as f64) * 0.2;
            Some((score, matched_positions))
        } else {
            None
        }
    }

    /// Try fuzzy matching
    fn try_fuzzy_match(&self, query: &str, target: &str) -> Option<(f64, Vec<usize>)> {
        let query_chars: Vec<char> = query.chars().collect();
        let target_chars: Vec<char> = target.chars().collect();

        let mut matched_positions = Vec::new();
        let mut query_idx = 0;
        let mut consecutive_matches = 0;
        let mut max_consecutive = 0;

        for (target_idx, &target_char) in target_chars.iter().enumerate() {
            if query_idx >= query_chars.len() {
                break;
            }

            if target_char == query_chars[query_idx] {
                matched_positions.push(target_idx);
                query_idx += 1;
                consecutive_matches += 1;
                max_consecutive = max_consecutive.max(consecutive_matches);
            } else {
                consecutive_matches = 0;
            }
        }

        if query_idx == query_chars.len() {
            // Calculate score based on:
            // - Match ratio
            // - Consecutive matches bonus
            // - Early matches bonus
            let match_ratio = query_chars.len() as f64 / target_chars.len() as f64;
            let consecutive_bonus = max_consecutive as f64 / query_chars.len() as f64 * 0.3;
            let early_bonus = if !matched_positions.is_empty() && matched_positions[0] < 3 {
                0.1
            } else {
                0.0
            };

            let score = (match_ratio * 0.6 + consecutive_bonus + early_bonus).min(1.0);
            Some((score, matched_positions))
        } else {
            None
        }
    }
}

impl Default for FuzzyMatcher {
    fn default() -> Self {
        Self::new(false) // Case insensitive by default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let matcher = FuzzyMatcher::default();
        let result = matcher.match_string("test", "test").unwrap();
        assert_eq!(result.match_type, MatchType::Exact);
        assert_eq!(result.score, 1.0);
    }

    #[test]
    fn test_prefix_match() {
        let matcher = FuzzyMatcher::default();
        let result = matcher.match_string("main", "main.rs").unwrap();
        assert_eq!(result.match_type, MatchType::Prefix);
        assert!(result.score > 0.8);
    }

    #[test]
    fn test_fuzzy_match() {
        let matcher = FuzzyMatcher::default();
        let result = matcher.match_string("mr", "main.rs").unwrap();
        // "mr" matches "main.rs" at word boundaries (m=main, r=rs), so it's WordStart
        assert_eq!(result.match_type, MatchType::WordStart);
        assert!(result.score > 0.0);

        // Test a true fuzzy match
        let result = matcher.match_string("mn", "main.rs").unwrap();
        assert_eq!(result.match_type, MatchType::Fuzzy);
        assert!(result.score > 0.0);
    }
}
