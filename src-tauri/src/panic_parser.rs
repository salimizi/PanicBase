use regex::Regex;

#[derive(Debug, Clone)]
pub struct ParsedPanicLog {
    pub panic_type: String,
    pub keywords: Vec<String>,
    pub missing_sensors: Vec<String>,
}

pub fn parse_panic_log(log: &str) -> ParsedPanicLog {
    let lower = log.to_lowercase();
    let mut keywords: Vec<String> = Vec::new();

    let known_keywords = [
        "thermalmonitord",
        "watchdog",
        "mic1",
        "mic2",
        "prs0",
        "tg0b",
        "tg0v",
        "ans2",
        "baseband",
        "applebcmwlan",
        "smc panic",
        "aop panic",
    ];

    for keyword in known_keywords {
        if lower.contains(keyword) {
            keywords.push(keyword.to_string());
        }
    }

    let missing_sensors = extract_missing_sensors(log);
    for sensor in &missing_sensors {
        if !keywords.iter().any(|k| k == sensor) {
            keywords.push(sensor.clone());
        }
    }

    let panic_type = if lower.contains("thermalmonitord") {
        "thermalmonitord".to_string()
    } else if lower.contains("watchdog") {
        "userspace_watchdog".to_string()
    } else if lower.contains("smc panic") {
        "smc_panic".to_string()
    } else if lower.contains("aop panic") {
        "aop_panic".to_string()
    } else if lower.contains("baseband") {
        "baseband".to_string()
    } else {
        "unknown".to_string()
    };

    ParsedPanicLog {
        panic_type,
        keywords,
        missing_sensors,
    }
}

fn push_missing_tokens(line: &str, sensors: &mut Vec<String>, seen: &mut std::collections::HashSet<String>) {
    for part in line.split(|c: char| c == ',' || c == ';' || c.is_whitespace()) {
        let cleaned: String = part
            .trim()
            .trim_matches(|c: char| c == '[' || c == ']' || c == '"' || c == '\'')
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_lowercase();
        if cleaned.len() >= 2 && cleaned.len() <= 24 && seen.insert(cleaned.clone()) {
            sensors.push(cleaned);
        }
    }
}

fn extract_missing_sensors(log: &str) -> Vec<String> {
    let re = Regex::new(r"(?i)missing sensor\(s\)?:?\s*([^\n\r]+)").unwrap();
    let mut sensors = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for caps in re.captures_iter(log) {
        if let Some(group) = caps.get(1) {
            push_missing_tokens(group.as_str(), &mut sensors, &mut seen);
        }
    }

    sensors
}

pub(crate) fn extract_missing_sensors_last(log: &str) -> Vec<String> {
    let re = Regex::new(r"(?i)missing sensor\(s\)?:?\s*([^\n\r]+)").unwrap();
    let mut sensors = Vec::new();
    let mut seen = std::collections::HashSet::new();
    if let Some(caps) = re.captures_iter(log).last() {
        if let Some(group) = caps.get(1) {
            push_missing_tokens(group.as_str(), &mut sensors, &mut seen);
        }
    }
    sensors
}
