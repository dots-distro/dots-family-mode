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
    domain_categories: std::collections::HashMap<String, String>,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self {
            domain_rules: HashSet::new(),
            url_patterns: Vec::new(),
            category_blocks: HashSet::new(),
            category_allows: HashSet::new(),
            domain_categories: std::collections::HashMap::new(),
        }
    }

    pub fn load_default_rules(&mut self) -> Result<()> {
        // Add default blocked domains (adult content)
        let adult_domains = [
            "pornhub.com",
            "xvideos.com",
            "xnxx.com",
            "redtube.com",
            "youporn.com",
            "tube8.com",
            "spankwire.com",
            "keezmovies.com",
            "extremetube.com",
            "onlyfans.com",
        ];

        for domain in &adult_domains {
            self.domain_rules.insert(domain.to_string());
            self.domain_categories.insert(domain.to_string(), "adult".to_string());
        }

        // Gambling sites
        let gambling_domains = [
            "gambling.com",
            "casino.com",
            "bet365.com",
            "pokerstars.com",
            "888casino.com",
            "betway.com",
            "draftkings.com",
            "fanduel.com",
        ];

        for domain in &gambling_domains {
            self.domain_rules.insert(domain.to_string());
            self.domain_categories.insert(domain.to_string(), "gambling".to_string());
        }

        // Violent/harmful content
        let violence_domains = ["4chan.org", "8kun.top", "bestgore.com"];

        for domain in &violence_domains {
            self.domain_rules.insert(domain.to_string());
            self.domain_categories.insert(domain.to_string(), "violence".to_string());
        }

        // Social media (optional blocking)
        let social_domains =
            ["facebook.com", "instagram.com", "tiktok.com", "snapchat.com", "twitter.com", "x.com"];

        for domain in &social_domains {
            self.domain_categories.insert(domain.to_string(), "social".to_string());
        }

        // Educational sites (always allow)
        let educational_domains = [
            "khanacademy.org",
            "coursera.org",
            "edx.org",
            "mit.edu",
            "stanford.edu",
            "wikipedia.org",
            "scratch.mit.edu",
            "code.org",
        ];

        for domain in &educational_domains {
            self.domain_categories.insert(domain.to_string(), "educational".to_string());
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
        self.add_url_pattern(r"(?i).*sex.*", FilterAction::Block, "Adult content")?;
        self.add_url_pattern(r"(?i).*casino.*", FilterAction::Block, "Gambling")?;
        self.add_url_pattern(r"(?i).*bet.*", FilterAction::Block, "Gambling")?;
        self.add_url_pattern(r"(?i).*gambling.*", FilterAction::Block, "Gambling")?;
        self.add_url_pattern(r"(?i).*gore.*", FilterAction::Block, "Violent content")?;
        self.add_url_pattern(r"(?i).*violence.*", FilterAction::Block, "Violent content")?;

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

        // Check if domain has a category
        let domain_category = self.domain_categories.get(domain);

        // Check category-based allow list first (highest priority)
        if let Some(category) = domain_category {
            if self.category_allows.contains(category) {
                return FilterDecision {
                    action: FilterAction::Allow,
                    reason: format!("Domain {} is in allowed category: {}", domain, category),
                    rule_id: Some(format!("category-allow:{}", category)),
                    category: Some(category.clone()),
                };
            }
        }

        // Check explicit domain block rules
        if self.domain_rules.contains(domain) {
            let category = domain_category.cloned();
            return FilterDecision {
                action: FilterAction::Block,
                reason: format!("Domain {} is explicitly blocked", domain),
                rule_id: Some(format!("domain:{}", domain)),
                category,
            };
        }

        // Check category-based block list
        if let Some(category) = domain_category {
            if self.category_blocks.contains(category) {
                return FilterDecision {
                    action: FilterAction::Block,
                    reason: format!("Domain {} is in blocked category: {}", domain, category),
                    rule_id: Some(format!("category-block:{}", category)),
                    category: Some(category.clone()),
                };
            }
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

    /// Get the category for a domain, if known
    pub fn get_domain_category(&self, domain: &str) -> Option<&String> {
        self.domain_categories.get(domain)
    }

    /// Add a domain to a specific category
    pub fn categorize_domain(&mut self, domain: &str, category: &str) {
        self.domain_categories.insert(domain.to_string(), category.to_string());
    }

    /// Get all domains in a category
    pub fn get_domains_in_category(&self, category: &str) -> Vec<&String> {
        self.domain_categories
            .iter()
            .filter(|(_, cat)| cat.as_str() == category)
            .map(|(domain, _)| domain)
            .collect()
    }

    /// Get all blocked domains
    #[allow(dead_code)]
    pub fn get_blocked_domains(&self) -> Vec<&String> {
        self.domain_rules.iter().collect()
    }

    /// Get all URL patterns
    #[allow(dead_code)]
    pub fn get_url_patterns(&self) -> Vec<(&str, &FilterAction, &String)> {
        self.url_patterns
            .iter()
            .map(|(regex, action, reason)| (regex.as_str(), action, reason))
            .collect()
    }

    /// Get blocked categories
    #[allow(dead_code)]
    pub fn get_blocked_categories(&self) -> Vec<&String> {
        self.category_blocks.iter().collect()
    }

    /// Get allowed categories
    #[allow(dead_code)]
    pub fn get_allowed_categories(&self) -> Vec<&String> {
        self.category_allows.iter().collect()
    }

    /// Check if a category is blocked
    #[allow(dead_code)]
    pub fn is_category_blocked(&self, category: &str) -> bool {
        self.category_blocks.contains(category)
    }

    /// Check if a category is explicitly allowed
    #[allow(dead_code)]
    pub fn is_category_allowed(&self, category: &str) -> bool {
        self.category_allows.contains(category)
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

    #[test]
    fn test_category_blocking() {
        let mut engine = RuleEngine::new();

        // Add domain to adult category
        engine.categorize_domain("badsite.com", "adult");
        engine.domain_rules.insert("badsite.com".to_string());
        engine.category_blocks.insert("adult".to_string());

        let decision = engine.evaluate_url("https://badsite.com/page");
        matches!(decision.action, FilterAction::Block);
        assert!(decision.category.is_some());
        assert_eq!(decision.category.unwrap(), "adult");
    }

    #[test]
    fn test_category_allow_override() {
        let mut engine = RuleEngine::new();

        // Add educational site
        engine.categorize_domain("khanacademy.org", "educational");
        engine.category_allows.insert("educational".to_string());

        let decision = engine.evaluate_url("https://khanacademy.org/math");
        matches!(decision.action, FilterAction::Allow);
        assert!(decision.reason.contains("educational"));
    }

    #[test]
    fn test_load_default_rules() {
        let mut engine = RuleEngine::new();
        engine.load_default_rules().unwrap();

        // Verify adult content is blocked
        let decision = engine.evaluate_url("https://pornhub.com");
        matches!(decision.action, FilterAction::Block);

        // Verify gambling is blocked
        let decision = engine.evaluate_url("https://bet365.com");
        matches!(decision.action, FilterAction::Block);

        // Verify educational sites are categorized
        assert_eq!(engine.get_domain_category("khanacademy.org"), Some(&"educational".to_string()));
    }

    #[test]
    fn test_pattern_matching() {
        let mut engine = RuleEngine::new();
        engine.load_default_rules().unwrap();

        // URL with "porn" in path should be blocked
        let decision = engine.evaluate_url("https://example.com/content/pornographic");
        matches!(decision.action, FilterAction::Block);

        // URL with "casino" in subdomain should be blocked
        let decision = engine.evaluate_url("https://casino.example.com/games");
        matches!(decision.action, FilterAction::Block);
    }

    #[test]
    fn test_get_domains_in_category() {
        let mut engine = RuleEngine::new();
        engine.load_default_rules().unwrap();

        let adult_domains = engine.get_domains_in_category("adult");
        assert!(!adult_domains.is_empty());
        assert!(adult_domains.contains(&&"pornhub.com".to_string()));

        let edu_domains = engine.get_domains_in_category("educational");
        assert!(!edu_domains.is_empty());
        assert!(edu_domains.contains(&&"khanacademy.org".to_string()));
    }
}
