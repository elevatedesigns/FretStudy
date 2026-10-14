//! Reading level check for the kid-simple explanations (target: about 5th grade).

/// Rough syllable count (vowel groups, minus a silent final "e").
pub fn syllables(word: &str) -> usize {
    let w: String = word
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .collect::<String>()
        .to_lowercase();
    if w.is_empty() {
        return 0;
    }
    let is_vowel = |c: char| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y');
    let mut count = 0;
    let mut prev = false;
    for c in w.chars() {
        let v = is_vowel(c);
        if v && !prev {
            count += 1;
        }
        prev = v;
    }
    if w.ends_with('e') && !w.ends_with("le") && count > 1 {
        count -= 1;
    }
    count.max(1)
}

/// Flesch–Kincaid grade level.
pub fn grade_level(text: &str) -> f32 {
    let sentences = text
        .split(['.', '!', '?'])
        .filter(|s| s.chars().any(|c| c.is_alphabetic()))
        .count()
        .max(1);
    let words: Vec<&str> = text
        .split_whitespace()
        .filter(|w| w.chars().any(|c| c.is_alphanumeric()))
        .collect();
    if words.is_empty() {
        return 0.0;
    }
    let syl: usize = words.iter().map(|w| syllables(w)).sum();
    0.39 * (words.len() as f32 / sentences as f32) + 11.8 * (syl as f32 / words.len() as f32)
        - 15.59
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_syllables() {
        assert_eq!(syllables("guitar"), 2);
        assert_eq!(syllables("strum"), 1);
        assert_eq!(syllables("make"), 1);
        assert_eq!(syllables("metronome"), 3);
    }

    #[test]
    fn simple_text_scores_low() {
        assert!(grade_level("The cat sat on the mat. It was a good cat.") < 2.0);
        assert!(
            grade_level(
                "Notwithstanding considerable organizational complexity, institutional responsibilities necessitate comprehensive evaluation."
            ) > 12.0
        );
    }
}
