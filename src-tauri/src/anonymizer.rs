use regex::Regex;

pub fn anonymize_panic_log(log: &str) -> String {
    let mut cleaned = log.to_string();

    let patterns = [
        (r"(?i)(serial(?: number|\s*nÂ°)?\s*[:=]\s*)[A-Za-z0-9-]+", "$1[REDACTED]"),
        (r"(?i)(imei\s*[:=]\s*)[0-9]+", "$1[REDACTED]"),
        (r"(?i)(udid\s*[:=]\s*)[A-Za-z0-9-]+", "$1[REDACTED]"),
        (r"(?i)(device name\s*[:=]\s*).+", "$1[REDACTED]"),
        (r"(?i)(/users?/|\\users\\)[^\\\r\n/]+(\\|/)", "${1}[REDACTED_USER]${2}"),
    ];

    for (pattern, replacement) in patterns {
        let re = Regex::new(pattern).unwrap();
        cleaned = re.replace_all(&cleaned, replacement).to_string();
    }

    cleaned
}
