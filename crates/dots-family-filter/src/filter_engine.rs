use anyhow::Result;
use dots_family_common::types::Activity;
use dots_family_proto::daemon::FamilyDaemonProxy;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use url::Url;
use zbus::Connection;

use crate::config::FilterConfig;
use crate::rules::{FilterAction, FilterDecision, RuleEngine};

pub struct FilterEngine {
    config: FilterConfig,
    rule_engine: Arc<RwLock<RuleEngine>>,
    daemon_proxy: Option<FamilyDaemonProxy<'static>>,
}

impl FilterEngine {
    pub async fn new(config: FilterConfig) -> Result<Self> {
        info!("Initializing Filter Engine");

        let mut rule_engine = RuleEngine::new();
        rule_engine.load_default_rules()?;

        let daemon_proxy = if config.daemon.check_permissions {
            Self::connect_to_daemon(&config.daemon.dbus_interface).await
        } else {
            None
        };

        Ok(Self { config, rule_engine: Arc::new(RwLock::new(rule_engine)), daemon_proxy })
    }

    async fn connect_to_daemon(interface: &str) -> Option<FamilyDaemonProxy<'static>> {
        match Connection::system().await {
            Ok(conn) => match FamilyDaemonProxy::new(&conn).await {
                Ok(proxy) => {
                    debug!("Connected to daemon via DBus: {}", interface);
                    Some(proxy)
                }
                Err(e) => {
                    warn!("Failed to connect to daemon: {}. Running in standalone mode.", e);
                    None
                }
            },
            Err(e) => {
                warn!("Failed to connect to system bus: {}. Running in standalone mode.", e);
                None
            }
        }
    }

    pub async fn evaluate_request(&self, url: &str, method: &str) -> Result<FilterDecision> {
        debug!("Evaluating request: {} {}", method, url);

        if !self.config.filtering.enabled {
            return Ok(FilterDecision {
                action: FilterAction::Allow,
                reason: "Filtering disabled".to_string(),
                rule_id: None,
                category: None,
            });
        }

        // Check with daemon if permissions are enabled
        if let Some(ref proxy) = self.daemon_proxy {
            if let Ok(parsed_url) = Url::parse(url) {
                if let Some(domain) = parsed_url.host_str() {
                    match proxy.check_application_allowed(domain).await {
                        Ok(false) => {
                            return Ok(FilterDecision {
                                action: FilterAction::Block,
                                reason: format!("Domain {} blocked by active profile", domain),
                                rule_id: Some("daemon_profile".to_string()),
                                category: Some("profile_policy".to_string()),
                            });
                        }
                        Ok(true) => {
                            debug!("Domain {} allowed by active profile", domain);
                        }
                        Err(e) => {
                            warn!("Failed to check with daemon: {}", e);
                        }
                    }
                }
            }
        }

        let rule_engine = self.rule_engine.read().await;
        let mut decision = rule_engine.evaluate_url(url);

        if self.config.filtering.safe_search_enforcement {
            if let Some(safe_url) = rule_engine.enforce_safe_search(url) {
                if safe_url != url {
                    decision.reason = format!("Safe search enforced: {}", decision.reason);
                }
            }
        }

        if self.config.daemon.log_activity {
            self.log_activity(url, &decision).await;
        }

        Ok(decision)
    }

    pub async fn rewrite_url_for_safe_search(&self, url: &str) -> Option<String> {
        if !self.config.filtering.safe_search_enforcement {
            return None;
        }

        let rule_engine = self.rule_engine.read().await;
        rule_engine.enforce_safe_search(url)
    }

    #[allow(dead_code)]
    pub async fn get_block_page_content(&self, url: &str, reason: &str) -> String {
        let domain = Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(String::from))
            .unwrap_or_else(|| "unknown".to_string());

        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Content Blocked - DOTS Family Mode</title>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            margin: 0;
            padding: 0;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            color: #333;
        }}
        .container {{
            background: white;
            padding: 2rem;
            border-radius: 12px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.2);
            text-align: center;
            max-width: 500px;
            margin: 1rem;
        }}
        .icon {{
            font-size: 4rem;
            margin-bottom: 1rem;
            color: #e74c3c;
        }}
        h1 {{
            color: #2c3e50;
            margin-bottom: 1rem;
            font-size: 1.8rem;
        }}
        .reason {{
            background: #f8f9fa;
            padding: 1rem;
            border-radius: 6px;
            margin: 1rem 0;
            border-left: 4px solid #e74c3c;
        }}
        .domain {{
            font-family: monospace;
            background: #ecf0f1;
            padding: 0.5rem;
            border-radius: 4px;
            margin: 0.5rem 0;
        }}
        .footer {{
            margin-top: 2rem;
            font-size: 0.9rem;
            color: #7f8c8d;
        }}
        .back-button {{
            background: #3498db;
            color: white;
            border: none;
            padding: 0.8rem 1.5rem;
            border-radius: 6px;
            cursor: pointer;
            font-size: 1rem;
            margin-top: 1rem;
            transition: background 0.2s;
        }}
        .back-button:hover {{
            background: #2980b9;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="icon">🛡️</div>
        <h1>Content Blocked</h1>
        <p>Access to this website has been restricted by DOTS Family Mode.</p>
        
        <div class="reason">
            <strong>Reason:</strong> {reason}
        </div>
        
        <div class="domain">
            <strong>Domain:</strong> {domain}
        </div>
        
        <button class="back-button" onclick="history.back()">
            ← Go Back
        </button>
        
        <div class="footer">
            <p>If you believe this is an error, please contact your parent or guardian.</p>
            <small>DOTS Family Mode • Content Protection</small>
        </div>
    </div>
</body>
</html>"#,
            reason = reason
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&#x27;"),
            domain = domain
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&#x27;")
        )
    }

    async fn log_activity(&self, url: &str, _decision: &FilterDecision) {
        if let Some(ref proxy) = self.daemon_proxy {
            let activity = Activity {
                id: uuid::Uuid::new_v4(),
                profile_id: uuid::Uuid::nil(),
                timestamp: chrono::Utc::now(),
                activity_type: dots_family_common::types::ActivityType::WebBrowsing {
                    url: url.to_string(),
                },
                application: Some("web-filter".to_string()),
                window_title: Some(url.to_string()),
                duration_seconds: 0,
            };

            let activity_json = serde_json::to_string(&activity).unwrap_or_default();
            if let Err(e) = proxy.report_activity(&activity_json).await {
                warn!("Failed to log activity to daemon: {}", e);
            }
        }
    }

    #[allow(dead_code)]
    pub async fn reload_rules(&self) -> Result<()> {
        info!("Reloading filter rules");
        let mut rule_engine = self.rule_engine.write().await;
        *rule_engine = RuleEngine::new();
        rule_engine.load_default_rules()?;
        info!("Filter rules reloaded successfully");
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn add_custom_rule(
        &self,
        pattern: &str,
        action: FilterAction,
        reason: &str,
    ) -> Result<()> {
        debug!("Adding custom rule: {} -> {:?}", pattern, action);
        let mut rule_engine = self.rule_engine.write().await;
        rule_engine.add_url_pattern(pattern, action, reason)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn is_filtering_enabled(&self) -> bool {
        self.config.filtering.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FilterConfig;

    async fn create_test_engine() -> FilterEngine {
        let config = FilterConfig::default();
        FilterEngine::new(config).await.unwrap()
    }

    #[tokio::test]
    async fn test_filter_engine_initialization() {
        let engine = create_test_engine().await;
        assert!(engine.is_filtering_enabled());
    }

    #[tokio::test]
    async fn test_safe_search_enforcement() {
        let engine = create_test_engine().await;

        let google_url = "https://google.com/search?q=test";
        let rewritten = engine.rewrite_url_for_safe_search(google_url).await;

        assert!(rewritten.is_some());
        assert!(rewritten.unwrap().contains("safe=active"));
    }

    #[tokio::test]
    async fn test_block_page_generation() {
        let engine = create_test_engine().await;

        let content = engine.get_block_page_content("https://blocked.com", "Adult content").await;

        assert!(content.contains("Content Blocked"));
        assert!(content.contains("Adult content"));
        assert!(content.contains("blocked.com"));
    }

    #[tokio::test]
    async fn test_custom_rule_addition() {
        let engine = create_test_engine().await;

        engine
            .add_custom_rule(r".*badsite.*", FilterAction::Block, "Custom block rule")
            .await
            .unwrap();

        let decision = engine.evaluate_request("https://badsite.com/path", "GET").await.unwrap();

        matches!(decision.action, FilterAction::Block);
    }
}
