use common::log::LogLevel;

pub fn parse_level(line: &str) -> LogLevel {
    let upper = line.to_uppercase();

    if contains_word(&upper, "FATAL") || contains_word(&upper, "CRIT") {
        LogLevel::Fatal
    } else if contains_word(&upper, "ERROR") || contains_word(&upper, "ERR") {
        LogLevel::Error
    } else if contains_word(&upper, "WARN") || contains_word(&upper, "WARNING") {
        LogLevel::Warn
    } else if contains_word(&upper, "DEBUG") || contains_word(&upper, "DBG") {
        LogLevel::Debug
    } else if contains_word(&upper, "TRACE") {
        LogLevel::Trace
    } else {
        LogLevel::Info
    }
}

fn contains_word(haystack: &str, word: &str) -> bool {
    for (i, _) in haystack.match_indices(word) {
        let before_ok = i == 0 || !haystack.as_bytes()[i - 1].is_ascii_alphanumeric();
        let after = i + word.len();
        let after_ok =
            after >= haystack.len() || !haystack.as_bytes()[after].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
    }
    false
}
