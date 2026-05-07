/// Violation detection logic for constraint violations
use crate::ast::*;
use crate::token::Token;

pub struct ViolationDetector {
    violations: Vec<ConstraintViolation>,
}

impl ViolationDetector {
    pub fn new() -> Self {
        ViolationDetector {
            violations: Vec::new(),
        }
    }

    pub fn detect_traditional_for_loop(
        &mut self,
        tokens: &[Token],
        start_pos: usize,
    ) -> bool {
        // Look ahead from current position to detect traditional for loop pattern
        // Pattern: for ( [type] identifier [=|;] ...
        let mut pos = start_pos;

        // Skip 'for' keyword and '('
        while pos < tokens.len() && !matches!(&tokens[pos], Token::Operator { value, .. } if value == "(") {
            pos += 1;
        }
        pos += 1; // skip '('

        // Skip optional 'const'
        if pos < tokens.len() {
            if let Token::Keyword { value, .. } = &tokens[pos] {
                if value == "const" {
                    pos += 1;
                }
            }
        }

        // Check for type keyword
        let has_type = pos < tokens.len() && is_type_keyword(&tokens[pos]);
        if !has_type {
            return false;
        }
        pos += 1;

        // Skip identifier
        if pos < tokens.len() && matches!(&tokens[pos], Token::Identifier { .. }) {
            pos += 1;
        } else {
            return false;
        }

        // Check next operator
        if pos < tokens.len() {
            if let Token::Operator { value, .. } = &tokens[pos] {
                // Traditional for: = or ;
                // Range-based for: :
                if value == "=" || value == ";" {
                    let line = tokens[start_pos].line();
                    self.violations.push(ConstraintViolation {
                        violation_type: "traditional_for_loop".to_string(),
                        line,
                        start_char: tokens[start_pos].column(),
                        end_char: tokens[start_pos].column() + 3,
                        message: "Traditional for loops not supported - use range-based for (for (auto& item : container)) instead".to_string(),
                    });
                    return true;
                }
            }
        }

        false
    }

    pub fn detect_raw_pointer_usage(&mut self, ast: &SourceFile) {
        // Walk through the AST and detect raw pointer usage
        for item in &ast.items {
            self.check_item_for_pointers(item);
        }
    }

    fn check_item_for_pointers(&mut self, item: &Item) {
        match item {
            Item::Function {
                return_type,
                parameters,
                body,
                line,
                ..
            } => {
                // Check return type
                if return_type.contains('*') {
                    self.violations.push(ConstraintViolation {
                        violation_type: "raw_pointer_usage".to_string(),
                        line: *line,
                        start_char: 0,
                        end_char: return_type.len(),
                        message: format!(
                            "Raw pointer return type '{}' not allowed - use std::vector, std::unique_ptr, or std::shared_ptr instead",
                            return_type
                        ),
                    });
                }

                // Check parameters
                for param in parameters {
                    if param.r#type.contains('*') {
                        self.violations.push(ConstraintViolation {
                            violation_type: "raw_pointer_usage".to_string(),
                            line: param.line,
                            start_char: 0,
                            end_char: param.r#type.len(),
                            message: format!(
                                "Raw pointer parameter '{}' not allowed - use std::vector, const reference, or pass by value",
                                param.r#type
                            ),
                        });
                    }
                }

                // Check body
                for stmt in body {
                    self.check_statement_for_pointers(stmt);
                }
            }
            Item::Class { members, line, .. } | Item::Struct { members, line, .. } => {
                for member in members {
                    match member {
                        ClassMember::Function {
                            return_type,
                            parameters,
                            ..
                        } => {
                            if return_type.contains('*') {
                                self.violations.push(ConstraintViolation {
                                    violation_type: "raw_pointer_usage".to_string(),
                                    line: *line,
                                    start_char: 0,
                                    end_char: return_type.len(),
                                    message: "Raw pointer return type not allowed".to_string(),
                                });
                            }
                            for param in parameters {
                                if param.r#type.contains('*') {
                                    self.violations.push(ConstraintViolation {
                                        violation_type: "raw_pointer_usage".to_string(),
                                        line: param.line,
                                        start_char: 0,
                                        end_char: param.r#type.len(),
                                        message: "Raw pointer parameter not allowed".to_string(),
                                    });
                                }
                            }
                        }
                        ClassMember::Variable { r#type, line, .. } => {
                            if r#type.contains('*') {
                                self.violations.push(ConstraintViolation {
                                    violation_type: "raw_pointer_usage".to_string(),
                                    line: *line,
                                    start_char: 0,
                                    end_char: r#type.len(),
                                    message: "Raw pointer member variable not allowed".to_string(),
                                });
                            }
                        }
                    }
                }
            }
            Item::Namespace { items, .. } => {
                for nested_item in items {
                    self.check_item_for_pointers(nested_item);
                }
            }
            Item::GlobalVariable { r#type, line, .. } => {
                if r#type.contains('*') {
                    self.violations.push(ConstraintViolation {
                        violation_type: "raw_pointer_usage".to_string(),
                        line: *line,
                        start_char: 0,
                        end_char: r#type.len(),
                        message: "Raw pointer global variable not allowed - use std::vector, std::unique_ptr, or std::shared_ptr".to_string(),
                    });
                }
            }
            _ => {}
        }
    }

    fn check_statement_for_pointers(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDecl {
                r#type,
                line,
                initializer,
                ..
            } => {
                if r#type.contains('*') {
                    self.violations.push(ConstraintViolation {
                        violation_type: "raw_pointer_usage".to_string(),
                        line: *line,
                        start_char: 0,
                        end_char: r#type.len(),
                        message: "Raw pointer variable declaration not allowed - use std::vector, std::unique_ptr, or std::shared_ptr".to_string(),
                    });
                }
                if let Some(expr) = initializer {
                    self.check_expression_for_pointers(expr);
                }
            }
            Statement::Block { statements } => {
                for s in statements {
                    self.check_statement_for_pointers(s);
                }
            }
            Statement::If { body, else_body, .. } => {
                for s in body {
                    self.check_statement_for_pointers(s);
                }
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        self.check_statement_for_pointers(s);
                    }
                }
            }
            Statement::While { body, .. } => {
                for s in body {
                    self.check_statement_for_pointers(s);
                }
            }
            Statement::For { body, .. } => {
                for s in body {
                    self.check_statement_for_pointers(s);
                }
            }
            _ => {}
        }
    }

    fn check_expression_for_pointers(&mut self, _expr: &Expression) {
        // Check for pointer dereference (*ptr) and address-of operator (&var)
        // This is more complex and would need to walk the expression tree
    }

    pub fn get_violations(self) -> Vec<ConstraintViolation> {
        self.violations
    }
}

impl Default for ViolationDetector {
    fn default() -> Self {
        Self::new()
    }
}

fn is_type_keyword(token: &Token) -> bool {
    matches!(
        token,
        Token::Keyword { value, .. } if matches!(
            value.as_str(),
            "int" | "float" | "double" | "char" | "bool" | "void" | "auto"
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_traditional_for_loop() {
        let source = "for (int i = 0; i < 10; i++) {}";
        let mut tokenizer = crate::token::Tokenizer::new(source);
        let tokens = tokenizer.tokenize().unwrap();

        let mut detector = ViolationDetector::new();
        let found = detector.detect_traditional_for_loop(&tokens, 0);

        assert!(found);
        assert_eq!(detector.violations.len(), 1);
        assert_eq!(detector.violations[0].violation_type, "traditional_for_loop");
    }

    #[test]
    fn test_detect_range_based_for_loop() {
        let source = "for (auto& x : container) {}";
        let mut tokenizer = crate::token::Tokenizer::new(source);
        let tokens = tokenizer.tokenize().unwrap();

        let mut detector = ViolationDetector::new();
        let found = detector.detect_traditional_for_loop(&tokens, 0);

        assert!(!found);
    }

    #[test]
    fn test_detect_raw_pointer_parameter() {
        let source = "void func(int* ptr) {}";
        let mut tokenizer = crate::token::Tokenizer::new(source);
        let tokens = tokenizer.tokenize().unwrap();
        let mut parser = crate::parser::Parser::new(tokens);
        let ast = parser.parse();

        let mut detector = ViolationDetector::new();
        detector.detect_raw_pointer_usage(&ast);

        assert!(detector.violations.iter().any(|v| v.violation_type == "raw_pointer_usage"));
    }
}
