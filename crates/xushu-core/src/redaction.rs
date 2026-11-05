use std::sync::LazyLock;

use regex::{Regex, RegexBuilder};

/// 脱敏结果。`flags` 记录命中的规则名称，供证据摘要与 UI 展示使用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionResult {
    pub text: String,
    pub flags: Vec<String>,
}

struct Rule {
    name: &'static str,
    pattern: Regex,
    replacement: &'static str,
}

static RULES: LazyLock<Vec<Rule>> = LazyLock::new(|| {
    vec![
        Rule {
            name: "private_key",
            pattern: RegexBuilder::new(
                r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z0-9 ]*PRIVATE KEY-----",
            )
            .multi_line(true)
            .build()
            .expect("private key regex"),
            replacement: "[REDACTED PRIVATE KEY]",
        },
        Rule {
            name: "authorization_header",
            pattern: RegexBuilder::new(
                r#"(?i)(authorization\s*[:=]\s*)(?:bearer|basic|token)\s+[^\s"']+"#,
            )
            .build()
            .expect("authorization regex"),
            replacement: "$1[REDACTED]",
        },
        Rule {
            name: "bearer_token",
            pattern: RegexBuilder::new(r"(?i)(bearer\s+)[A-Za-z0-9._~+/=-]{8,}")
                .build()
                .expect("bearer regex"),
            replacement: "$1[REDACTED]",
        },
        Rule {
            name: "api_key_prefix",
            pattern: RegexBuilder::new(
                r"(?i)\b(?:sk-[A-Za-z0-9_-]{8,}|ghp_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|AKIA[0-9A-Z]{16}|xox[baprs]-[A-Za-z0-9-]{10,})\b",
            )
            .build()
            .expect("api key prefix regex"),
            replacement: "[REDACTED]",
        },
        Rule {
            name: "credential_assignment",
            pattern: RegexBuilder::new(
                r#"(?i)\b(api[_-]?key|apikey|access[_-]?token|auth[_-]?token|refresh[_-]?token|client[_-]?secret|secret|password|passwd|pwd|token)\b(\s*[:=]\s*)(?:"[^"]*"|'[^']*'|[^\s"']+)"#,
            )
            .build()
            .expect("credential assignment regex"),
            replacement: "$1$2[REDACTED]",
        },
        Rule {
            name: "connection_string",
            pattern: RegexBuilder::new(
                r"(?i)\b(postgres(?:ql)?|mysql|mongodb(?:\+srv)?|redis|amqp)://[^:\s/@]+:[^@\s]+@",
            )
            .build()
            .expect("connection string regex"),
            replacement: "$1://[REDACTED]@",
        },
    ]
});

/// 对验证输出做模式化脱敏。
///
/// 这是基于模式的最佳努力，不是 100% 防泄漏保证；调用方仍应限制输出大小，
/// 并在 UI 中明确说明原始敏感内容可能无法被完全识别。
pub fn redact(input: &str) -> RedactionResult {
    let mut text = input.to_owned();
    let mut flags = Vec::new();

    for rule in RULES.iter() {
        if rule.pattern.is_match(&text) {
            text = rule
                .pattern
                .replace_all(&text, rule.replacement)
                .into_owned();
            flags.push(rule.name.to_owned());
        }
    }

    RedactionResult { text, flags }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_common_credentials() {
        let input = "API_KEY=sk-test-secret-123456\nAuthorization: Bearer abcdefghijklmnop\npassword: hunter2";
        let result = redact(input);
        assert!(!result.text.contains("sk-test-secret-123456"));
        assert!(!result.text.contains("abcdefghijklmnop"));
        assert!(!result.text.contains("hunter2"));
        assert!(result.text.contains("[REDACTED]"));
        assert!(result.flags.contains(&"credential_assignment".to_owned()));
        assert!(result.flags.contains(&"api_key_prefix".to_owned()));
    }

    #[test]
    fn redacts_private_key_blocks() {
        let input =
            "before\n-----BEGIN RSA PRIVATE KEY-----\nMIIEow\n-----END RSA PRIVATE KEY-----\nafter";
        let result = redact(input);
        assert!(!result.text.contains("MIIEow"));
        assert!(result.text.contains("[REDACTED PRIVATE KEY]"));
    }

    #[test]
    fn leaves_ordinary_output_unchanged() {
        let input = "test result: ok. 12 passed; 0 failed";
        let result = redact(input);
        assert_eq!(result.text, input);
        assert!(result.flags.is_empty());
    }
}
