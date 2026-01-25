use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterAction {
    Allow,
    Block,
    Warn,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterRuleType {
    Domain,
    Url,
    Category,
    Pattern,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRule {
    pub id: String,
    pub rule_type: FilterRuleType,
    pub pattern: String,
    pub action: FilterAction,
    pub category: Option<String>,
    pub reason: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct FilterDecision {
    pub action: FilterAction,
    pub reason: String,
    #[allow(dead_code)]
    pub rule_id: Option<String>,
    #[allow(dead_code)]
    pub category: Option<String>,
}

pub struct RuleEngine {
    domain_rules: HashSet<String>,
    url_patterns: Vec<(Regex, FilterAction, String)>,
    category_blocks: HashSet<String>,
    category_allows: HashSet<String>,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self {
            domain_rules: HashSet::new(),
            url_patterns: Vec::new(),
            category_blocks: HashSet::new(),
            category_allows: HashSet::new(),
        }
    }

    pub fn load_default_rules(&mut self) -> Result<()> {
        // Add default blocked domains
        let blocked_domains = [
            "pornhub.com",
            "xvideos.com",
            "xnxx.com",
            "redtube.com",
            "gambling.com",
            "casino.com",
            "bet365.com",
            "4chan.org",
            "8kun.top",
        ];

        for domain in &blocked_domains {
            self.domain_rules.insert(domain.to_string());
        }

        // Add default blocked categories
        self.category_blocks.insert("adult".to_string());
        self.category_blocks.insert("gambling".to_string());
        self.category_blocks.insert("violence".to_string());
        self.category_blocks.insert("drugs".to_string());

        // Add default allowed categories
        self.category_allows.insert("educational".to_string());
        self.category_allows.insert("children".to_string());
        self.category_allows.insert("reference".to_string());

        // Add URL pattern rules
        self.add_url_pattern(r"(?i).*porn.*", FilterAction::Block, "Adult content")?;
        self.add_url_pattern(r"(?i).*xxx.*", FilterAction::Block, "Adult content")?;
        self.add_url_pattern(r"(?i).*casino.*", FilterAction::Block, "Gambling")?;
        self.add_url_pattern(r"(?i).*bet.*", FilterAction::Block, "Gambling")?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn add_domain_rule(&mut self, domain: &str, action: FilterAction) {
        match action {
            FilterAction::Block => {
                self.domain_rules.insert(domain.to_string());
            }
            FilterAction::Allow => {
                self.domain_rules.remove(domain);
            }
            FilterAction::Warn => {
                // For now, treat warn as allow
                self.domain_rules.remove(domain);
            }
        }
    }

    pub fn add_url_pattern(
        &mut self,
        pattern: &str,
        action: FilterAction,
        reason: &str,
    ) -> Result<()> {
        let regex = Regex::new(pattern)?;
        self.url_patterns.push((regex, action, reason.to_string()));
        Ok(())
    }

    #[allow(dead_code)]
    pub fn add_category_rule(&mut self, category: &str, action: FilterAction) {
        match action {
            FilterAction::Block => {
                self.category_blocks.insert(category.to_string());
                self.category_allows.remove(category);
            }
            FilterAction::Allow => {
                self.category_allows.insert(category.to_string());
                self.category_blocks.remove(category);
            }
            FilterAction::Warn => {
                // For now, treat warn as allow
                self.category_allows.insert(category.to_string());
                self.category_blocks.remove(category);
            }
        }
    }

    pub fn evaluate_url(&self, url: &str) -> FilterDecision {
        let parsed_url = match Url::parse(url) {
            Ok(url) => url,
            Err(_) => {
                return FilterDecision {
                    action: FilterAction::Block,
                    reason: "Invalid URL format".to_string(),
                    rule_id: None,
                    category: None,
                };
            }
        };

        let domain = parsed_url.host_str().unwrap_or("");

        // Check domain rules
        if self.domain_rules.contains(domain) {
            return FilterDecision {
                action: FilterAction::Block,
                reason: format!("Domain {} is blocked", domain),
                rule_id: Some(format!("domain:{}", domain)),
                category: None,
            };
        }

        // Check URL pattern rules
        for (pattern, action, reason) in &self.url_patterns {
            if pattern.is_match(url) {
                return FilterDecision {
                    action: action.clone(),
                    reason: reason.clone(),
                    rule_id: Some(format!("pattern:{}", pattern.as_str())),
                    category: None,
                };
            }
        }

        // Default allow
        FilterDecision {
            action: FilterAction::Allow,
            reason: "No matching rules, allowing".to_string(),
            rule_id: None,
            category: None,
        }
    }

    pub fn enforce_safe_search(&self, url: &str) -> Option<String> {
        let parsed_url = match Url::parse(url) {
            Ok(url) => url,
            Err(_) => return None,
        };

        let domain = parsed_url.host_str().unwrap_or("");

        match domain {
            "google.com" | "www.google.com" => {
                let mut safe_url = parsed_url;
                safe_url.query_pairs_mut().append_pair("safe", "active");
                Some(safe_url.to_string())
            }
            "bing.com" | "www.bing.com" => {
                let mut safe_url = parsed_url;
                safe_url.query_pairs_mut().append_pair("adlt", "strict");
                Some(safe_url.to_string())
            }
            "duckduckgo.com" | "www.duckduckgo.com" => {
                let mut safe_url = parsed_url;
                safe_url.query_pairs_mut().append_pair("safe-search", "strict");
                Some(safe_url.to_string())
            }
            "youtube.com" | "www.youtube.com" => {
                let mut safe_url = parsed_url;
                safe_url.query_pairs_mut().append_pair("restrict_mode", "strict");
                Some(safe_url.to_string())
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_blocking() {
        let mut engine = RuleEngine::new();
        engine.add_domain_rule("example.com", FilterAction::Block);

        let decision = engine.evaluate_url("https://example.com/path");
        matches!(decision.action, FilterAction::Block);
        assert!(decision.reason.contains("example.com"));
    }

    #[test]
    fn test_url_pattern_blocking() {
        let mut engine = RuleEngine::new();
        engine.add_url_pattern(r".*casino.*", FilterAction::Block, "Gambling").unwrap();

        let decision = engine.evaluate_url("https://example.com/casino/games");
        matches!(decision.action, FilterAction::Block);
        assert!(decision.reason.contains("Gambling"));
    }

    #[test]
    fn test_safe_search_enforcement() {
        let engine = RuleEngine::new();

        let google_url = "https://google.com/search?q=test";
        let safe_url = engine.enforce_safe_search(google_url).unwrap();
        assert!(safe_url.contains("safe=active"));

        let youtube_url = "https://youtube.com/watch?v=123";
        let safe_youtube = engine.enforce_safe_search(youtube_url).unwrap();
        assert!(safe_youtube.contains("restrict_mode=strict"));
    }

    #[test]
    fn test_default_allow() {
        let engine = RuleEngine::new();
        let decision = engine.evaluate_url("https://wikipedia.org/");
        matches!(decision.action, FilterAction::Allow);
    }
}
