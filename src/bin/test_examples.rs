/// Simple test binary for DietC++ LSP - tests examples without full parsing
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::collections::HashMap;

use dietcpp_lsp::token::{Tokenizer, Token};
use dietcpp_lsp::config::DietCppConfig;

// Color codes
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const CYAN: &str = "\x1b[36m";
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";

struct ViolationContext {
    line_num: usize,
    violation_type: String,
    message: String,
    code_line: String,
    char_start: usize,
    char_end: usize,
    suggestion: String,
}

fn main() {
    let start_time = Instant::now();

    println!("\n{}╔════════════════════════════════════════════════════════════════════╗{}", BLUE, RESET);
    println!("{}║              {}DietC++ LSP - Example Test Suite{}                   {}║{}", BLUE, BOLD, RESET, BLUE, RESET);
    println!("{}╚════════════════════════════════════════════════════════════════════╝{}\n", BLUE, RESET);

    // Load configuration
    let config_dir = Path::new("/home/abuss/Work/devel/dietcpp");
    let config = match DietCppConfig::load(config_dir) {
        Ok(cfg) => {
            println!("{}✓ Loaded configuration from {}{}",  GREEN, config_dir.display(), RESET);
            cfg
        }
        Err(e) => {
            println!("{}⚠ Using default configuration: {}{}",  YELLOW, e, RESET);
            DietCppConfig::default()
        }
    };
    println!("  • Severity: {}", config.general.severity);
    println!("  • Preprocessor check: {}", if config.rules.preprocessor_directives { "enabled" } else { "disabled" });
    println!("  • Forbidden keywords check: {}", if config.rules.forbidden_keywords { "enabled" } else { "disabled" });
    println!("  • Traditional for loops check: {}", if config.rules.traditional_for_loops { "enabled" } else { "disabled" });
    println!();

    // Discover C++ files
    let examples_dir = Path::new("/home/abuss/Work/devel/dietcpp/cpp-examples");
    let mut files = discover_cpp_files(examples_dir);
    files.sort();

    if files.is_empty() {
        println!("{}✗ No C++ files found in {}{}",  RED, examples_dir.display(), RESET);
        return;
    }

    println!("{}Found {} C++ example files{}\n", CYAN, files.len(), RESET);

    let mut all_results = Vec::new();
    let mut violation_type_counts: HashMap<String, usize> = HashMap::new();
    let mut total_violations = 0;

    // Test each file
    for (idx, filepath) in files.iter().enumerate() {
        print!("{}Testing {}/{}... ", CYAN, idx + 1, files.len());
        std::io::Write::flush(&mut std::io::stdout()).ok();

        let result = test_file(&filepath);
        
        if let Some(violations) = &result {
            for violation in violations {
                *violation_type_counts
                    .entry(violation.violation_type.clone())
                    .or_insert(0) += 1;
                total_violations += 1;
            }
        }
        
        println!("{}✓{}", GREEN, RESET);
        all_results.push((filepath.clone(), result));
    }

    println!();

    // Print results for each file
    for (filepath, violations_opt) in all_results.iter() {
        let filename = filepath
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        println!("{}📄 {}{}", CYAN, filename, RESET);
        println!("{}──────────────────────────────────────────────────────────────────────{}", BLUE, RESET);

        if let Some(content) = fs::read_to_string(filepath).ok() {
            let lines = content.lines().count();
            println!("{}✅ Read: {} lines{}", GREEN, lines, RESET);

            if let Some(violations) = violations_opt {
                println!("{}❌ Violations: {}{}", RED, violations.len(), RESET);
                println!();

                for violation in violations {
                    print_violation(violation);
                }
            } else {
                println!("{}✓ No violations found{}", GREEN, RESET);
            }
        } else {
            println!("{}✗ Failed to read file{}", RED, RESET);
        }

        println!();
    }

    // Print summary
    print_summary(&all_results, violation_type_counts, total_violations, start_time.elapsed());
}

fn discover_cpp_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "cpp" || ext == "cc" || ext == "cxx" {
                        files.push(path);
                    }
                }
            }
        }
    }

    files
}

fn test_file(filepath: &Path) -> Option<Vec<ViolationContext>> {
    // Read file
    let content = fs::read_to_string(filepath).ok()?;
    let lines: Vec<&str> = content.lines().collect();

    // Tokenize
    let mut tokenizer = Tokenizer::new(&content);
    let tokens = tokenizer.tokenize().ok()?;

    let mut violations = Vec::new();
    
    // Use default config for now (test binary doesn't load config per file)
    let config = DietCppConfig::default();

    // Detect forbidden keywords
    for token in &tokens {
        if let Token::Keyword { value, line, column } = token {
            if config.is_keyword_forbidden(value) {
                if let Some(code_line) = lines.get(line.saturating_sub(1)) {
                    // Tokenizer uses 1-indexed columns, convert to 0-indexed
                    let char_start = column.saturating_sub(1);
                    violations.push(ViolationContext {
                        line_num: *line,
                        violation_type: "forbidden_keyword".to_string(),
                        message: format!("Forbidden keyword '{}' - use modern C++ instead", value),
                        code_line: code_line.to_string(),
                        char_start,
                        char_end: char_start + value.len(),
                        suggestion: "💡 Use modern C++ alternatives".to_string(),
                    });
                }
            }
        }
    }

    // Detect preprocessor directives
    for token in &tokens {
        if let Token::Preprocessor { text, line, column } = token {
            if let Some(code_line) = lines.get(line.saturating_sub(1)) {
                // Tokenizer uses 1-indexed columns, convert to 0-indexed
                let char_start = column.saturating_sub(1);
                violations.push(ViolationContext {
                    line_num: *line,
                    violation_type: "preprocessor_directive".to_string(),
                    message: "Preprocessor directives not supported - use C++ language features".to_string(),
                    code_line: code_line.to_string(),
                    char_start,
                    char_end: char_start + text.len(),
                    suggestion: "💡 Use C++ language features instead".to_string(),
                });
            }
        }
    }

     // Detect traditional for loops (simple pattern matching)
     for (i, token) in tokens.iter().enumerate() {
         if let Token::Keyword { value, line, column } = token {
             if value == "for" {
                 // Look ahead for pattern: for ( int i = 0 ; i < ...
                 let mut paren_depth = 0;
                 let mut has_traditional = false;

                 for j in i+1..std::cmp::min(i+15, tokens.len()) {
                     match &tokens[j] {
                         Token::Operator { value, .. } if value == "(" => {
                             paren_depth += 1;
                         }
                         Token::Operator { value, .. } if value == ")" => {
                             paren_depth -= 1;
                             if paren_depth == 0 {
                                 break;
                             }
                         }
                         Token::Keyword { value, .. } if value == "int" || value == "auto" => {
                             // Check for assignment or colon
                             for k in j+1..std::cmp::min(j+5, tokens.len()) {
                                 if let Token::Operator { value, .. } = &tokens[k] {
                                     if value == "=" {
                                         has_traditional = true;
                                     } else if value == ":" {
                                         has_traditional = false;
                                     }
                                 }
                             }
                         }
                         _ => {}
                     }
                 }

                if has_traditional {
                    if let Some(code_line) = lines.get(line.saturating_sub(1)) {
                        // Tokenizer uses 1-indexed columns, convert to 0-indexed
                        let char_start = column.saturating_sub(1);
                        violations.push(ViolationContext {
                            line_num: *line,
                            violation_type: "traditional_for_loop".to_string(),
                            message: "Traditional for loops not supported - use range-based for".to_string(),
                            code_line: code_line.to_string(),
                            char_start,
                            char_end: char_start + 3,
                            suggestion: "💡 Use range-based for: for (auto& item : container)".to_string(),
                        });
                    }
                }
            }
        }
    }

    // Detect raw pointer usage (simple pattern: type* or &*)
    for (i, token) in tokens.iter().enumerate() {
        if let Token::Operator { value, line, column } = token {
            if value == "*" {
                // Check if this is a pointer in a declaration
                // Look for pattern: TYPE * where TYPE is a keyword or qualified type
                if i > 0 {
                    // Get previous token - must be a type keyword or identifier that's part of a type
                    let prev_is_type_keyword = match &tokens[i - 1] {
                        Token::Keyword { value: kw, .. } => matches!(kw.as_str(), 
                            "int" | "float" | "double" | "char" | "bool" | "void" | "auto" | 
                            "unsigned" | "signed" | "long" | "short" | "const"),
                        Token::Operator { value: op, .. } if op == "*" => true,  // For ** pointers
                        _ => false,
                    };

                    if prev_is_type_keyword {
                        // Check if next token suggests this is part of a declaration (or end of declaration)
                        let is_declaration = if i + 1 < tokens.len() {
                            match &tokens[i + 1] {
                                Token::Identifier { .. } => true,  // TYPE * varname
                                Token::Operator { value: op, .. } if op == "*" => true,  // TYPE ** (double pointer)
                                Token::Operator { value: op, .. } if op == ";" => true,  // TYPE *;
                                Token::Operator { value: op, .. } if op == "," => true,  // TYPE *, next param
                                Token::Operator { value: op, .. } if op == "(" => true,  // TYPE *param
                                Token::Operator { value: op, .. } if op == ")" => true,  // TYPE *)
                                _ => false,
                            }
                        } else {
                            true  // End of tokens
                        };

                        if is_declaration {
                            if let Some(code_line) = lines.get(line.saturating_sub(1)) {
                                // Find the start of the type (look backwards from the * for the type keyword)
                                let mut type_start = column.saturating_sub(1);
                                
                                // Look backwards to find where the type starts
                                let mut j = i as i32 - 1;
                                while j >= 0 {
                                    match &tokens[j as usize] {
                                        Token::Keyword { value: kw, column: kw_col, .. } if matches!(kw.as_str(), 
                                            "int" | "float" | "double" | "char" | "bool" | "void" | "auto" | 
                                            "unsigned" | "signed" | "long" | "short" | "const") => {
                                            type_start = kw_col.saturating_sub(1);
                                            break;
                                        }
                                        _ => {}
                                    }
                                    j -= 1;
                                    if j < i as i32 - 3 { break; }  // Don't look too far back
                                }

                                // Tokenizer uses 1-indexed columns, convert to 0-indexed
                                let char_end = column.saturating_sub(1) + 1;  // Mark the * included
                                violations.push(ViolationContext {
                                    line_num: *line,
                                    violation_type: "raw_pointer_usage".to_string(),
                                    message: "Raw pointer usage not allowed - use std::unique_ptr, std::shared_ptr, or pass by reference".to_string(),
                                    code_line: code_line.to_string(),
                                    char_start: type_start,
                                    char_end,
                                    suggestion: "💡 Use smart pointers (unique_ptr/shared_ptr) or pass by reference".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Detect address-of operator usage (&var, which creates a pointer)
    for (i, token) in tokens.iter().enumerate() {
        if let Token::Operator { value, line, column } = token {
            if value == "&" {
                // Check if this is an address-of operator (creating a pointer), not a reference type declaration
                // Address-of operator patterns:
                // 1. &var or &x in assignment: ... = &var
                // 2. func(&var) in function call
                // 3. &(expression)
                
                // Type declaration patterns (should NOT be flagged):
                // 1. TYPE & or TYPE&: int& ref = x;
                // 2. CONST TYPE &: const int& ref = x;
                // 3. Range-based for: for (auto& item : container)
                
                let is_address_of = if i > 0 && i + 1 < tokens.len() {
                    // If next token is an identifier or (, it might be address-of
                    match &tokens[i + 1] {
                        Token::Identifier { .. } => {
                            // Check if previous token is an operator like = or (
                            match &tokens[i - 1] {
                                Token::Operator { value: op, .. } if matches!(op.as_str(), "=" | "(" | "," | "{") => true,
                                _ => false,
                            }
                        }
                        Token::Operator { value: op, .. } if op == "(" => {
                            // &(...) - address-of with parentheses
                            match &tokens[i - 1] {
                                Token::Operator { value: op, .. } if matches!(op.as_str(), "=" | "(" | ",") => true,
                                _ => false,
                            }
                        }
                        _ => false,
                    }
                } else {
                    false
                };

                if is_address_of {
                    if let Some(code_line) = lines.get(line.saturating_sub(1)) {
                        // Tokenizer uses 1-indexed columns, convert to 0-indexed
                        let char_start = column.saturating_sub(1);
                        violations.push(ViolationContext {
                            line_num: *line,
                            violation_type: "address_of_operator".to_string(),
                            message: "Address-of operator creates pointers which are not allowed - use references instead".to_string(),
                            code_line: code_line.to_string(),
                            char_start,
                            char_end: char_start + 1,
                            suggestion: "💡 Use references (&) as type declarations or std::addressof() if needed".to_string(),
                        });
                    }
                }
            }
        }
    }

    // Sort by line number
    violations.sort_by_key(|v| v.line_num);

    if violations.is_empty() {
        None
    } else {
        Some(violations)
    }
}

fn print_violation(violation: &ViolationContext) {
    // Print header
    println!(
        "  {}Line {}  │ {}{}{}",
        "\x1b[37m", violation.line_num, RED, violation.violation_type, RESET
    );

    // Print separator
    println!("  {}────────┼─────────────────────────────────────────────────────────{}", BLUE, RESET);

    // Print code line with context
    println!("          │ {}", violation.code_line);

    // Print highlighting
    let mut highlight = String::from("          │ ");
    for (i, _) in violation.code_line.chars().enumerate() {
        if i >= violation.char_start && i < violation.char_end {
            highlight.push('^');
        } else if i < violation.char_start {
            highlight.push(' ');
        } else {
            break;
        }
    }
    println!("{}{}{}", RED, highlight, RESET);

    // Print message and suggestion
    println!("          │");
    println!("          │ {}{}{}", YELLOW, violation.message, RESET);
    println!("          │ {}{}{}", GREEN, violation.suggestion, RESET);
    println!();
}

fn print_summary(
    all_results: &[(PathBuf, Option<Vec<ViolationContext>>)],
    violation_counts: HashMap<String, usize>,
    total_violations: usize,
    elapsed: std::time::Duration,
) {
    let total_files = all_results.len();
    let mut files_passed = 0;
    let mut files_with_violations = 0;

    let mut passed_files = Vec::new();
    let mut violation_files = Vec::new();

    for (filepath, violations_opt) in all_results {
        let filename = filepath
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        if violations_opt.is_none() {
            files_passed += 1;
            passed_files.push(filename);
        } else {
            files_with_violations += 1;
            let count = violations_opt.as_ref().map(|v| v.len()).unwrap_or(0);
            violation_files.push((filename, count));
        }
    }

    println!(
        "{}═══════════════════════════════════════════════════════════════════════{}",
        BLUE, RESET
    );
    println!("{}📊 SUMMARY{}", BOLD, RESET);
    println!(
        "{}═══════════════════════════════════════════════════════════════════════{}",
        BLUE, RESET
    );
    println!();

    println!("{}Files Tested: {}{}", CYAN, total_files, RESET);

    if files_passed > 0 {
        println!("{}├─ {}✅ Passed (no violations):  {}{}", BLUE, GREEN, files_passed, RESET);
        for filename in &passed_files {
            println!("{}│  ├─ {}{}", BLUE, filename, RESET);
        }
    }

    if files_with_violations > 0 {
        println!("{}├─ {}⚠️  With Violations:  {}{}", BLUE, YELLOW, files_with_violations, RESET);
        for (filename, count) in &violation_files {
            println!(
                "{}│  ├─ {} {}({}){} violations{}",
                BLUE, filename, RED, count, RESET, RESET
            );
        }
    }

    println!();
    println!("{}Total Violations: {}{}", BOLD, total_violations, RESET);

    if !violation_counts.is_empty() {
        let mut sorted_violations: Vec<_> = violation_counts.iter().collect();
        sorted_violations.sort_by_key(|(_, count)| std::cmp::Reverse(**count));

        for (violation_type, count) in sorted_violations {
            let icon = match violation_type.as_str() {
                "traditional_for_loop" => "🔁",
                "raw_pointer_usage" => "🔌",
                "forbidden_keyword" => "⛔",
                "preprocessor_directive" => "#️⃣",
                _ => "⚠️",
            };
            println!("  {} {}: {}{}{}", icon, violation_type, RED, count, RESET);
        }
    }

    println!();
    println!("{}Execution Time: {:.2}ms{}", CYAN, elapsed.as_millis(), RESET);
    println!(
        "{}═══════════════════════════════════════════════════════════════════════{}\n",
        BLUE, RESET
    );
}
