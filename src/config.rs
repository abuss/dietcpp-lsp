/// Configuration system for DietC++ LSP
/// Supports JSON and TOML formats
/// Can be configured per-project or per-workspace

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Main configuration struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DietCppConfig {
    /// General settings
    pub general: GeneralConfig,
    
    /// Violation rules - what should be flagged
    pub rules: RulesConfig,
    
    /// Language features - what's allowed/disallowed
    pub features: FeaturesConfig,
    
    /// Naming conventions
    pub naming: NamingConfig,
}

/// General configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Project name
    #[serde(default)]
    pub project_name: String,
    
    /// Enable/disable all checks
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// Severity level: "error", "warning", "info"
    #[serde(default = "default_severity")]
    pub severity: String,
    
    /// Only report violations above this line (for performance)
    #[serde(default)]
    pub max_violations_per_file: usize, // 0 = unlimited
}

/// Rules configuration - which violations to report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesConfig {
    /// Preprocessor directives (#include, #define, etc.)
    #[serde(default = "default_true")]
    pub preprocessor_directives: bool,
    
    /// Forbidden keywords
    #[serde(default = "default_true")]
    pub forbidden_keywords: bool,
    
    /// Traditional C-style for loops
    #[serde(default = "default_true")]
    pub traditional_for_loops: bool,
    
    /// Raw pointer usage
    #[serde(default = "default_true")]
    pub raw_pointers: bool,
    
    /// Custom rule enablement
    #[serde(default)]
    pub custom: HashMap<String, bool>,
}

/// Language features configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesConfig {
    /// Allowed keywords
    #[serde(default = "default_allowed_keywords")]
    pub allowed_keywords: Vec<String>,
    
    /// Forbidden keywords (can override defaults)
    #[serde(default = "default_forbidden_keywords")]
    pub forbidden_keywords: Vec<String>,
    
    /// Allow specific preprocessor directives (e.g., ["#pragma", "#define"])
    #[serde(default)]
    pub allowed_preprocessor: Vec<String>,
    
    /// Allow smart pointers
    #[serde(default = "default_true")]
    pub allow_smart_pointers: bool,
    
    /// Allow standard library
    #[serde(default = "default_true")]
    pub allow_std_lib: bool,
    
    /// Allow const references
    #[serde(default = "default_true")]
    pub allow_const_references: bool,
    
    /// Allow rvalue references
    #[serde(default = "default_true")]
    pub allow_rvalue_references: bool,
    
    /// Allow range-based for loops
    #[serde(default = "default_true")]
    pub allow_range_based_for: bool,
    
    /// Allow templates
    #[serde(default = "default_true")]
    pub allow_templates: bool,
    
    /// Allow exceptions (throw/try/catch)
    #[serde(default)]
    pub allow_exceptions: bool,
    
    /// Allow namespace
    #[serde(default = "default_true")]
    pub allow_namespace: bool,
    
    /// Allow classes
    #[serde(default = "default_true")]
    pub allow_classes: bool,
    
    /// Allow structs
    #[serde(default = "default_true")]
    pub allow_structs: bool,
    
    /// Allow inline functions
    #[serde(default)]
    pub allow_inline: bool,
    
    /// Allow static keyword
    #[serde(default)]
    pub allow_static: bool,
    
    /// Allow extern keyword
    #[serde(default)]
    pub allow_extern: bool,
    
    /// Allow goto
    #[serde(default)]
    pub allow_goto: bool,
}

/// Naming conventions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamingConfig {
    /// File naming pattern (e.g., "snake_case", "camelCase", "PascalCase")
    #[serde(default)]
    pub file_naming: Option<String>,
    
    /// Function naming pattern
    #[serde(default)]
    pub function_naming: Option<String>,
    
    /// Variable naming pattern
    #[serde(default)]
    pub variable_naming: Option<String>,
    
    /// Constant naming pattern
    #[serde(default)]
    pub constant_naming: Option<String>,
    
    /// Class naming pattern
    #[serde(default)]
    pub class_naming: Option<String>,
}

// Default implementations
fn default_true() -> bool {
    true
}

fn default_severity() -> String {
    "error".to_string()
}

fn default_allowed_keywords() -> Vec<String> {
    vec![
        "int", "float", "double", "char", "bool", "void", "auto",
        "const", "unsigned", "signed", "long", "short",
        "class", "struct", "enum", "union", "namespace",
        "if", "else", "while", "for", "do", "switch", "case", "default",
        "return", "break", "continue",
        "public", "private", "protected", "using",
        "nullptr", "true", "false",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

fn default_forbidden_keywords() -> Vec<String> {
    vec![
        "friend", "register", "throw", "try", "catch", "final", 
        "virtual", "goto", "mutable", "extern", "inline", 
        "static", "typedef",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

impl Default for DietCppConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                project_name: "dietcpp_project".to_string(),
                enabled: true,
                severity: "error".to_string(),
                max_violations_per_file: 0,
            },
            rules: RulesConfig {
                preprocessor_directives: true,
                forbidden_keywords: true,
                traditional_for_loops: true,
                raw_pointers: true,
                custom: HashMap::new(),
            },
            features: FeaturesConfig {
                allowed_keywords: default_allowed_keywords(),
                forbidden_keywords: default_forbidden_keywords(),
                allowed_preprocessor: vec![],
                allow_smart_pointers: true,
                allow_std_lib: true,
                allow_const_references: true,
                allow_rvalue_references: true,
                allow_range_based_for: true,
                allow_templates: true,
                allow_exceptions: false,
                allow_namespace: true,
                allow_classes: true,
                allow_structs: true,
                allow_inline: false,
                allow_static: false,
                allow_extern: false,
                allow_goto: false,
            },
            naming: NamingConfig {
                file_naming: None,
                function_naming: None,
                variable_naming: None,
                constant_naming: None,
                class_naming: None,
            },
        }
    }
}

impl DietCppConfig {
    /// Load configuration from JSON file
    pub fn from_json<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Invalid JSON config: {}", e))
    }

    /// Load configuration from TOML file
    pub fn from_toml<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        toml::from_str(&content)
            .map_err(|e| format!("Invalid TOML config: {}", e))
    }

    /// Auto-detect and load configuration
    /// Looks for: .dietcpprc.json, .dietcpprc.toml, dietcpp.json, dietcpp.toml
    pub fn load<P: AsRef<Path>>(project_root: P) -> Result<Self, String> {
        let root = project_root.as_ref();
        
        // Try different config filenames
        let config_names = vec![
            ".dietcpprc.json",
            ".dietcpprc.toml",
            "dietcpp.json",
            "dietcpp.toml",
            ".dietcpp.json",
            ".dietcpp.toml",
        ];

        for name in config_names {
            let path = root.join(name);
            if path.exists() {
                return if name.ends_with(".json") {
                    Self::from_json(&path)
                } else {
                    Self::from_toml(&path)
                };
            }
        }

        // Return default if no config found
        Ok(Self::default())
    }

    /// Save configuration to JSON file
    pub fn to_json<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Serialization error: {}", e))?;
        fs::write(path, json)
            .map_err(|e| format!("Failed to write config: {}", e))
    }

    /// Save configuration to TOML file
    pub fn to_toml<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let toml = toml::to_string_pretty(self)
            .map_err(|e| format!("Serialization error: {}", e))?;
        fs::write(path, toml)
            .map_err(|e| format!("Failed to write config: {}", e))
    }

    /// Check if a keyword is forbidden
    pub fn is_keyword_forbidden(&self, keyword: &str) -> bool {
        self.features.forbidden_keywords.contains(&keyword.to_string())
    }

    /// Check if a keyword is allowed
    pub fn is_keyword_allowed(&self, keyword: &str) -> bool {
        self.features.allowed_keywords.contains(&keyword.to_string())
    }

    /// Check if preprocessor directive is allowed
    pub fn is_preprocessor_allowed(&self, directive: &str) -> bool {
        if self.features.allowed_preprocessor.is_empty() {
            // If no whitelist, disallow all
            false
        } else {
            self.features.allowed_preprocessor.contains(&directive.to_string())
        }
    }

    /// Check if a specific rule is enabled
    pub fn is_rule_enabled(&self, rule: &str) -> bool {
        match rule {
            "preprocessor_directives" => self.rules.preprocessor_directives,
            "forbidden_keywords" => self.rules.forbidden_keywords,
            "traditional_for_loops" => self.rules.traditional_for_loops,
            "raw_pointers" => self.rules.raw_pointers,
            _ => self.rules.custom.get(rule).copied().unwrap_or(false),
        }
    }

    /// Convert to LSP initializationOptions format
    pub fn to_lsp_options(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::json!({}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DietCppConfig::default();
        assert!(config.general.enabled);
        assert!(config.rules.preprocessor_directives);
        assert!(config.rules.forbidden_keywords);
    }

    #[test]
    fn test_is_forbidden_keyword() {
        let config = DietCppConfig::default();
        assert!(config.is_keyword_forbidden("virtual"));
        assert!(config.is_keyword_forbidden("static"));
        assert!(!config.is_keyword_forbidden("int"));
    }

    #[test]
    fn test_is_keyword_allowed() {
        let config = DietCppConfig::default();
        assert!(config.is_keyword_allowed("int"));
        assert!(config.is_keyword_allowed("float"));
        assert!(!config.is_keyword_allowed("virtual"));
    }

    #[test]
    fn test_is_rule_enabled() {
        let config = DietCppConfig::default();
        assert!(config.is_rule_enabled("preprocessor_directives"));
        assert!(config.is_rule_enabled("forbidden_keywords"));
    }
}
