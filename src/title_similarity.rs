//! Small title differences are acceptable; a shared word alone is not enough.

/// Scores already cleaned title keys. Exact matches rank above close matches.
/// Work stays bounded even when a source supplies an unusually long title.
pub fn score(expected: &str, candidate: &str) -> Option<i32> {
    if expected.is_empty() || candidate.is_empty() {
        return None;
    }
    if expected == candidate {
        return Some(40);
    }
    if numbers(expected) != numbers(candidate) {
        return None;
    }
    if roman_suffix(expected) != roman_suffix(candidate) {
        return None;
    }
    let compact = |text: &str| {
        text.chars()
            .filter(|c| !c.is_whitespace())
            .collect::<Vec<_>>()
    };
    let left = compact(expected);
    let right = compact(candidate);
    if left == right {
        return Some(25);
    }
    // Short names such as "Love" and "Live", or "Song" and "Song 2",
    // need more than approximate spelling to establish the same song.
    let longest = left.len().max(right.len());
    if left.len().min(right.len()) < 8 || longest > 256 {
        return None;
    }
    let mut left_words: Vec<_> = expected.split_whitespace().collect();
    let mut right_words: Vec<_> = candidate.split_whitespace().collect();
    left_words.sort_unstable();
    right_words.sort_unstable();
    if left_words == right_words {
        return Some(20);
    }
    let limit = (longest * 15 / 100).clamp(1, 3);
    if left.len().abs_diff(right.len()) > limit {
        return None;
    }
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0; right.len() + 1];
    for (i, a) in left.iter().enumerate() {
        current[0] = i + 1;
        for (j, b) in right.iter().enumerate() {
            current[j + 1] = (previous[j] + usize::from(a != b))
                .min(previous[j + 1] + 1)
                .min(current[j] + 1);
        }
        if current.iter().copied().min().unwrap_or(usize::MAX) > limit {
            return None;
        }
        std::mem::swap(&mut previous, &mut current);
    }
    (previous[right.len()] <= limit).then_some(10)
}

fn numbers(text: &str) -> Vec<&str> {
    text.split(|c: char| !c.is_numeric())
        .filter(|part| !part.is_empty())
        .collect()
}

fn roman_suffix(text: &str) -> Option<&str> {
    text.split_whitespace()
        .last()
        .filter(|word| ["i", "ii", "iii", "iv", "v", "vi", "vii", "viii", "ix", "x"].contains(word))
}

#[cfg(test)]
mod tests {
    use super::score;

    #[test]
    fn similar_spelling_spacing_and_word_order_are_accepted() {
        for (left, right) in [
            ("i am machine", "i am machne"),
            ("painkiller", "painkiler"),
            ("never recover", "recover never"),
            ("dont stop", "don t stop"),
            ("whatever it takes", "whatever takes"),
        ] {
            assert!(score(left, right).is_some(), "{left} / {right}");
        }
    }

    #[test]
    fn exact_titles_rank_above_approximate_titles() {
        assert!(score("painkiller", "painkiller") > score("painkiller", "painkiler"));
    }

    #[test]
    fn sequels_short_names_and_shared_word_subsets_are_distinct() {
        for (left, right) in [
            ("song", "song 2"),
            ("never recover", "never recover 2"),
            ("song part 2", "song part 3"),
            ("never recover ii", "never recover iii"),
            ("love", "live"),
            ("love", "love story"),
            ("never recover", "never recover again"),
            ("i am machine", "i am human"),
        ] {
            assert!(score(left, right).is_none(), "{left} / {right}");
        }
    }

    #[test]
    fn empty_and_excessively_long_approximate_titles_are_rejected() {
        assert_eq!(score("", ""), None);
        assert_eq!(score(&"a".repeat(10000), &"b".repeat(10000)), None);
    }
}
