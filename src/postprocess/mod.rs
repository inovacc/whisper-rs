//! Pure, dependency-free text post-processing transforms.
//!
//! These functions operate purely on `&str` input (no model, no I/O, no
//! `unsafe`) and are always available regardless of feature flags.

/// Returns the numeric value of a single English cardinal-number word, or
/// `None` if `word` (already lowercased) is not a recognized number word.
fn number_word_value(word: &str) -> Option<u64> {
    let value = match word {
        "zero" => 0,
        "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        "eleven" => 11,
        "twelve" => 12,
        "thirteen" => 13,
        "fourteen" => 14,
        "fifteen" => 15,
        "sixteen" => 16,
        "seventeen" => 17,
        "eighteen" => 18,
        "nineteen" => 19,
        "twenty" => 20,
        "thirty" => 30,
        "forty" => 40,
        "fifty" => 50,
        "sixty" => 60,
        "seventy" => 70,
        "eighty" => 80,
        "ninety" => 90,
        "hundred" => 100,
        "thousand" => 1000,
        _ => return None,
    };
    Some(value)
}

/// Evaluates a maximal run of cardinal-number tokens (units/teens/tens/
/// hundred/thousand, with optional connecting "and") into its integer value.
fn evaluate_number_run(run: &[&str]) -> u64 {
    let mut total: u64 = 0;
    let mut current: u64 = 0;
    for tok in run {
        let low = tok.to_lowercase();
        if low == "and" {
            continue;
        }
        match number_word_value(&low) {
            Some(100) => {
                current = if current == 0 { 100 } else { current * 100 };
            }
            Some(1000) => {
                let multiplier = if current == 0 { 1 } else { current };
                total += multiplier * 1000;
                current = 0;
            }
            Some(v) => {
                current += v;
            }
            None => {}
        }
    }
    total + current
}

/// Converts spoken English cardinal number words in `text` into digits.
///
/// Handles integers built from units/teens/tens/hundred/thousand, including
/// "and" as a connector ("one hundred and five" -> "105"). Matching is
/// case-insensitive. Non-number words are left untouched. Whitespace between
/// tokens is normalized to single spaces.
pub fn normalize_numbers(text: &str) -> String {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let mut out: Vec<String> = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        let low = tokens[i].to_lowercase();
        if number_word_value(&low).is_some() {
            let mut run: Vec<&str> = Vec::new();
            let mut j = i;
            while j < tokens.len() {
                let jlow = tokens[j].to_lowercase();
                let is_number = number_word_value(&jlow).is_some();
                let is_and_connector = jlow == "and"
                    && !run.is_empty()
                    && j + 1 < tokens.len()
                    && number_word_value(&tokens[j + 1].to_lowercase()).is_some();
                if is_number || is_and_connector {
                    run.push(tokens[j]);
                    j += 1;
                } else {
                    break;
                }
            }
            let value = evaluate_number_run(&run);
            out.push(value.to_string());
            i = j;
        } else {
            out.push(tokens[i].to_string());
            i += 1;
        }
    }
    out.join(" ")
}

/// Collapses runs of 3 or more consecutive, case-insensitively identical
/// words down to a single occurrence (keeping the first occurrence's
/// casing). Runs of 2 are left unchanged.
pub fn collapse_repeats(text: &str) -> String {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let mut out: Vec<&str> = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        let mut j = i + 1;
        while j < tokens.len() && tokens[j].eq_ignore_ascii_case(tokens[i]) {
            j += 1;
        }
        let run_len = j - i;
        if run_len >= 3 {
            out.push(tokens[i]);
        } else {
            out.extend_from_slice(&tokens[i..j]);
        }
        i = j;
    }
    out.join(" ")
}

/// Returns the conservative filler-word list for a given language code, or
/// `None` if the language is not supported.
fn filler_words(lang: &str) -> Option<&'static [&'static str]> {
    match lang {
        "en" => Some(&["um", "uh", "er", "ah", "hmm"]),
        "es" => Some(&["eh", "este"]),
        _ => None,
    }
}

/// Removes standalone filler words for the given language from `text`.
/// Matching is whole-word and case-insensitive; resulting extra whitespace
/// is collapsed to single spaces. An unrecognized `lang` leaves `text`
/// unchanged.
pub fn remove_fillers(text: &str, lang: &str) -> String {
    let Some(fillers) = filler_words(lang) else {
        return text.to_string();
    };
    let tokens: Vec<&str> = text
        .split_whitespace()
        .filter(|tok| {
            let low = tok.to_lowercase();
            !fillers.contains(&low.as_str())
        })
        .collect();
    tokens.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_words_map_correctly() {
        assert_eq!(number_word_value("twenty"), Some(20));
        assert_eq!(number_word_value("nope"), None);
    }

    #[test]
    fn collapse_basic() {
        assert_eq!(collapse_repeats("the the the cat"), "the cat");
    }

    #[test]
    fn fillers_unknown_lang_noop() {
        assert_eq!(remove_fillers("hello", "xx"), "hello");
    }
}
