/// Recursive descent parser for DietC++ subset
use crate::ast::*;
use crate::token::Token;
use crate::config::DietCppConfig;

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
    violations: Vec<ConstraintViolation>,
    config: DietCppConfig,
}


const TYPE_KEYWORDS: &[&str] = &["int", "float", "double", "char", "bool", "void", "auto"];

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            position: 0,
            violations: Vec::new(),
            config: DietCppConfig::default(),
        }
    }

    pub fn with_config(tokens: Vec<Token>, config: DietCppConfig) -> Self {
        Parser {
            tokens,
            position: 0,
            violations: Vec::new(),
            config,
        }
    }

    pub fn config(&self) -> &DietCppConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: DietCppConfig) {
        self.config = config;
    }

    pub fn parse(&mut self) -> SourceFile {
        let mut ast = SourceFile::new();

        // Phase 1: Scan for forbidden keywords
        self.scan_forbidden_keywords(&mut ast);

        // Phase 2: Scan for preprocessor directives
        self.scan_preprocessor_directives(&mut ast);

         // Phase 3: Main parse
         self.position = 0;
         while !self.is_at_end() {
             if let Ok(item) = self.parse_item() {
                 ast.items.push(item);
             } else {
                 self.skip_to_next_statement_boundary();
             }
         }

          // Phase 4: AST-based detection (disabled in favor of more reliable token-based detection)
          // let mut violation_detector = ViolationDetector::new();
          // if self.config.rules.raw_pointers {
          //     violation_detector.detect_raw_pointer_usage(&ast);
          // }
          // let detector_violations = violation_detector.get_violations();

          // Add collected violations to AST
          ast.constraint_violations.append(&mut self.violations);
          // ast.constraint_violations.extend(detector_violations);
          
          // Phase 5: Token-based raw pointer detection for comprehensive coverage
          if self.config.rules.raw_pointers {
              self.detect_raw_pointers_from_tokens(&mut ast);
          }
          
          ast
    }

      fn scan_forbidden_keywords(&self, ast: &mut SourceFile) {
          // Skip if rule is disabled in config
          if !self.config.is_rule_enabled("forbidden_keywords") {
              return;
          }

          for token in &self.tokens {
              if let Token::Keyword { value, line, column } = token {
                  // Use config to check if keyword is forbidden
                  if self.config.is_keyword_forbidden(value) {
                      ast.forbidden_keywords.push(ForbiddenKeyword {
                          keyword: value.clone(),
                          line: *line,
                      });
                      // Also add as constraint violation so it appears in editor
                      // Tokenizer uses 1-indexed columns, VS Code expects 0-indexed
                      let start_col = column.saturating_sub(1);
                      ast.constraint_violations.push(ConstraintViolation {
                          violation_type: "forbidden_keyword".to_string(),
                          line: *line,
                          start_char: start_col,
                          end_char: start_col + value.len(),
                         message: format!("Forbidden keyword '{}' - use modern C++ instead", value),
                     });
                 }
             }
         }
     }

    fn scan_preprocessor_directives(&self, ast: &mut SourceFile) {
         // Skip if rule is disabled in config
         if !self.config.is_rule_enabled("preprocessor_directives") {
             return;
         }

         for token in &self.tokens {
             if let Token::Preprocessor { text, line, .. } = token {
                 // Extract directive (e.g., "#include" from "#include <stdio.h>")
                 let directive = text.split_whitespace().next().unwrap_or("#");
                 
                 // Check if this preprocessor directive is allowed in config
                 if !self.config.is_preprocessor_allowed(directive) {
                     ast.constraint_violations.push(ConstraintViolation {
                         violation_type: "preprocessor_directive".to_string(),
                         line: *line,
                         start_char: 0,
                         end_char: text.len(),
                         message: format!("Preprocessor directive '{}' not allowed", directive),
                     });
                 }
             }
         }
     }

    fn detect_raw_pointers_from_tokens(&self, ast: &mut SourceFile) {
        // Token-based detection for raw pointers (similar to test_examples.rs)
        for (i, token) in self.tokens.iter().enumerate() {
            if let Token::Operator { value, line, column } = token {
                if value == "*" {
                    // Check if this is a pointer in a declaration
                    if i > 0 {
                        let prev_is_type_keyword = match &self.tokens[i - 1] {
                            Token::Keyword { value: kw, .. } => matches!(kw.as_str(),
                                "int" | "float" | "double" | "char" | "bool" | "void" | "auto" |
                                "unsigned" | "signed" | "long" | "short" | "const"),
                            Token::Operator { value: op, .. } if op == "*" => true,  // For ** pointers
                            _ => false,
                        };

                        if prev_is_type_keyword {
                            // Check if next token suggests this is part of a declaration
                            let is_declaration = if i + 1 < self.tokens.len() {
                                match &self.tokens[i + 1] {
                                    Token::Identifier { .. } => true,  // TYPE * varname
                                    Token::Operator { value: op, .. } if op == "*" => true,  // TYPE ** (double pointer)
                                    Token::Operator { value: op, .. } if op == ";" => true,  // TYPE *;
                                    Token::Operator { value: op, .. } if op == "," => true,  // TYPE *, next param
                                    Token::Operator { value: op, .. } if op == "(" => true,  // TYPE *param (in function)
                                    Token::Operator { value: op, .. } if op == ")" => true,  // TYPE *)
                                    _ => false,
                                }
                            } else {
                                true  // End of tokens
                            };

                            if is_declaration {
                                // Find the start of the type (look backwards from the * for the type keyword)
                                let mut type_start = column.saturating_sub(1);

                                // Look backwards to find where the type starts
                                let mut j = i as i32 - 1;
                                while j >= 0 {
                                    match &self.tokens[j as usize] {
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
                                ast.constraint_violations.push(ConstraintViolation {
                                    violation_type: "raw_pointer_usage".to_string(),
                                    line: *line,
                                    start_char: type_start,
                                    end_char: char_end,
                                    message: "Raw pointer usage not allowed - use std::unique_ptr, std::shared_ptr, or pass by reference".to_string(),
                                });
                            }
                        }
                    }
                } else if value == "&" {
                    // Check if this is an address-of operator in an expression (not in a type declaration)
                    // Address-of operator follows = or ( in expressions like: int* p = &x; or func(&x);
                    if i > 0 {
                        let is_address_of = match &self.tokens[i - 1] {
                            Token::Operator { value: op, .. } if op == "=" || op == "(" || op == "," => true,
                            Token::Keyword { value: kw, .. } if kw == "return" => true,
                            _ => false,
                        };
                        
                        if is_address_of {
                            // Make sure the next token is an identifier (the variable being referenced)
                            if i + 1 < self.tokens.len() {
                                if matches!(&self.tokens[i + 1], Token::Identifier { .. }) {
                                    let char_pos = column.saturating_sub(1);  // Convert to 0-indexed
                                    ast.constraint_violations.push(ConstraintViolation {
                                        violation_type: "address_of_operator".to_string(),
                                        line: *line,
                                        start_char: char_pos,
                                        end_char: char_pos + 1,
                                        message: "Address-of operator creates pointers which are not allowed - use references instead".to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }


    // Helper methods
    fn is_at_end(&self) -> bool {
        matches!(self.peek(), Token::EOF)
    }

    fn peek(&self) -> &Token {
        if self.position < self.tokens.len() {
            &self.tokens[self.position]
        } else {
            &Token::EOF
        }
    }

    fn peek_ahead(&self, offset: usize) -> &Token {
        if self.position + offset < self.tokens.len() {
            &self.tokens[self.position + offset]
        } else {
            &Token::EOF
        }
    }

    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.position += 1;
        }
        self.tokens[self.position - 1].clone()
    }

    fn expect_keyword(&mut self, keyword: &str) -> bool {
        if let Token::Keyword { value, .. } = self.peek() {
            if value == keyword {
                self.advance();
                return true;
            }
        }
        false
    }

    fn expect_operator(&mut self, op: &str) -> bool {
        if let Token::Operator { value, .. } = self.peek() {
            if value == op {
                self.advance();
                return true;
            }
        }
        false
    }

    fn expect_identifier(&mut self) -> Option<String> {
        if let Token::Identifier { value, .. } = self.peek() {
            let id = value.clone();
            self.advance();
            return Some(id);
        }
        None
    }

    fn peek_keyword(&self, keyword: &str) -> bool {
        if let Token::Keyword { value, .. } = self.peek() {
            value == keyword
        } else {
            false
        }
    }

    fn peek_operator(&self, op: &str) -> bool {
        if let Token::Operator { value, .. } = self.peek() {
            value == op
        } else {
            false
        }
    }

    fn peek_identifier(&self) -> bool {
        matches!(self.peek(), Token::Identifier { .. })
    }

    fn skip_to_next_statement_boundary(&mut self) {
        // Safety: Always advance at least once to avoid infinite loops
        if !self.is_at_end() {
            let start_pos = self.position;
            
            while !self.is_at_end() {
                match self.peek() {
                    Token::Operator { value, .. } if value == ";" => {
                        self.advance();  // Consume the semicolon
                        break;
                    }
                    Token::Operator { value, .. } if value == "}" => {
                        // Don't consume '}' - let parent context handle it
                        // But ensure we advanced at least once
                        if self.position == start_pos {
                            self.advance();
                        }
                        break;
                    }
                    _ => {
                        self.advance();
                    }
                }
            }
        }
    }

    // Parsing functions
     fn parse_item(&mut self) -> Result<Item, String> {
         if self.expect_keyword("using") {
             self.parse_using_declaration()
         } else if self.expect_keyword("namespace") {
             self.parse_namespace_declaration()
         } else if self.expect_keyword("class") {
             self.parse_class_declaration()
         } else if self.expect_keyword("struct") {
             self.parse_struct_declaration()
         } else if self.expect_keyword("enum") {
             self.parse_enum_declaration()
         } else {
             // Try to parse as function declaration first
             let saved_pos = self.position;
             match self.parse_function_declaration() {
                 Ok(item) => Ok(item),
                 Err(_) => {
                     // If function declaration failed, try parsing as global variable
                     self.position = saved_pos;
                     self.parse_global_variable_declaration()
                 }
             }
         }
     }

    fn parse_using_declaration(&mut self) -> Result<Item, String> {
        let line = self.peek().line();
        if !self.expect_keyword("namespace") {
            return Err("Expected 'namespace' after 'using'".to_string());
        }
        let namespace = self.expect_identifier().ok_or("Expected namespace name")?;
        if !self.expect_operator(";") {
            return Err("Expected ';' after using declaration".to_string());
        }
        Ok(Item::UsingDeclaration { namespace, line })
    }

    fn parse_namespace_declaration(&mut self) -> Result<Item, String> {
        let line = self.peek().line();
        let name = self.expect_identifier().ok_or("Expected namespace name")?;
        if !self.expect_operator("{") {
            return Err("Expected '{' after namespace name".to_string());
        }

        let mut items = Vec::new();
        while !self.peek_operator("}") && !self.is_at_end() {
            if let Ok(item) = self.parse_item() {
                items.push(item);
            } else {
                self.skip_to_next_statement_boundary();
            }
        }

        if !self.expect_operator("}") {
            return Err("Expected '}' to close namespace".to_string());
        }

        Ok(Item::Namespace { name, items, line })
    }

    fn parse_class_declaration(&mut self) -> Result<Item, String> {
        let line = self.peek().line();
        let name = self.expect_identifier().ok_or("Expected class name")?;

        // Skip inheritance if present
        if self.expect_operator(":") {
            while !self.peek_operator("{") && !self.is_at_end() {
                self.advance();
            }
        }

        if !self.expect_operator("{") {
            return Err("Expected '{' after class name".to_string());
        }

        let mut members = Vec::new();
        while !self.peek_operator("}") && !self.is_at_end() {
            if let Ok(member) = self.parse_class_member() {
                members.push(member);
            } else {
                self.skip_to_next_statement_boundary();
            }
        }

        if !self.expect_operator("}") {
            return Err("Expected '}' to close class".to_string());
        }

        if !self.expect_operator(";") {
            // Optional semicolon after class
        }

        Ok(Item::Class { name, members, line })
    }

    fn parse_struct_declaration(&mut self) -> Result<Item, String> {
        let line = self.peek().line();
        let name = self.expect_identifier().ok_or("Expected struct name")?;

        if !self.expect_operator("{") {
            return Err("Expected '{' after struct name".to_string());
        }

        let mut members = Vec::new();
        while !self.peek_operator("}") && !self.is_at_end() {
            if let Ok(member) = self.parse_class_member() {
                members.push(member);
            } else {
                self.skip_to_next_statement_boundary();
            }
        }

        if !self.expect_operator("}") {
            return Err("Expected '}' to close struct".to_string());
        }

        if !self.expect_operator(";") {
            // Optional semicolon after struct
        }

        Ok(Item::Struct { name, members, line })
    }

    fn parse_enum_declaration(&mut self) -> Result<Item, String> {
        let line = self.peek().line();
        let name = self.expect_identifier().ok_or("Expected enum name")?;

        if !self.expect_operator("{") {
            return Err("Expected '{' after enum name".to_string());
        }

        let mut members = Vec::new();
        while !self.peek_operator("}") && !self.is_at_end() {
            let member_name = self.expect_identifier().ok_or("Expected enum member name")?;
            let value = if self.expect_operator("=") {
                if let Token::Number { value, .. } = self.peek() {
                    let num = value.clone();
                    self.advance();
                    Some(num)
                } else {
                    return Err("Expected number after '='".to_string());
                }
            } else {
                None
            };

            members.push(EnumMember {
                name: member_name,
                value,
                line: self.peek().line(),
            });

            if !self.expect_operator(",") {
                break;
            }
        }

        if !self.expect_operator("}") {
            return Err("Expected '}' to close enum".to_string());
        }

        if !self.expect_operator(";") {
            // Optional semicolon
        }

        Ok(Item::Enum { name, members, line })
    }

    fn parse_class_member(&mut self) -> Result<ClassMember, String> {
        // First, check if this is a visibility label
        if self.peek_keyword("public") || self.peek_keyword("private") || self.peek_keyword("protected") {
            let next_tok = self.peek_ahead(1);
            if matches!(next_tok, Token::Operator { value, .. } if value == ":") {
                // This is a visibility label, not a member - skip it
                self.advance();  // Skip visibility keyword
                self.expect_operator(":");
                return Err("Visibility label, not a member".to_string());
            }
        }

        let visibility = "public".to_string();  // Default visibility

        // Try to parse as variable or function
        let r#type = self.parse_type()?;
        let name = self.expect_identifier().ok_or("Expected member name")?;

        if self.expect_operator("(") {
            // Function member
            let mut parameters = Vec::new();
            while !self.peek_operator(")") && !self.is_at_end() {
                // Parameters can be empty
                if self.peek_operator(")") {
                    break;
                }
                
                let param_type = self.parse_type()?;
                let param_name = self.expect_identifier().ok_or("Expected parameter name")?;
                parameters.push(Parameter {
                    name: param_name,
                    r#type: param_type,
                    line: self.peek().line(),
                });

                if !self.expect_operator(",") {
                    break;
                }
            }

            if !self.expect_operator(")") {
                return Err("Expected ')' after parameters".to_string());
            }

            // Skip function specifiers like const, noexcept, override
            while self.peek_keyword("const") || self.peek_keyword("noexcept") || self.peek_keyword("override") || self.peek_keyword("override") {
                self.advance();
                // noexcept might have parameters, skip them
                if self.expect_operator("(") {
                    while !self.peek_operator(")") && !self.is_at_end() {
                        self.advance();
                    }
                    self.expect_operator(")");
                }
            }

            let mut body = Vec::new();
            if self.expect_operator("{") {
                while !self.peek_operator("}") && !self.is_at_end() {
                    if let Ok(stmt) = self.parse_statement() {
                        body.push(stmt);
                    } else {
                        self.skip_to_next_statement_boundary();
                    }
                }
                if !self.expect_operator("}") {
                    return Err("Expected '}' to close function".to_string());
                }
            } else if self.expect_operator(";") {
                // Function declaration without body
            }

            Ok(ClassMember::Function {
                visibility,
                return_type: r#type,
                name,
                parameters,
                body,
                line: self.peek().line(),
            })
        } else {
            // Variable member
            if !self.expect_operator(";") {
                return Err("Expected ';' after member variable".to_string());
            }
            Ok(ClassMember::Variable {
                visibility,
                r#type,
                name,
                line: self.peek().line(),
            })
        }
    }

     fn parse_function_declaration(&mut self) -> Result<Item, String> {
         use std::fs::OpenOptions;
         use std::io::Write;
         
         if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
             let _ = writeln!(log, "[PARSER] parse_function_declaration started at pos {}", self.position);
         }
         
         let line = self.peek().line();
         
         if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
             let _ = writeln!(log, "[PARSER] Parsing return type...");
         }
         let return_type = self.parse_type()?;
         if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
             let _ = writeln!(log, "[PARSER] Return type: {}", return_type);
         }
         
         if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
             let _ = writeln!(log, "[PARSER] Expecting identifier for function name...");
         }
         let name = self.expect_identifier().ok_or("Expected function name")?;
         if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
             let _ = writeln!(log, "[PARSER] Function name: {}", name);
         }

         if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
             let _ = writeln!(log, "[PARSER] Expecting '(' after function name...");
         }
         if !self.expect_operator("(") {
             if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
                 let _ = writeln!(log, "[PARSER] ERROR: No '(' found");
             }
             return Err("Expected '(' after function name".to_string());
         }
         if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
             let _ = writeln!(log, "[PARSER] Found '('");
         }

         let mut parameters = Vec::new();
         while !self.peek_operator(")") && !self.is_at_end() {
             if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
                 let _ = writeln!(log, "[PARSER] Parsing parameter at pos {}", self.position);
             }
             let param_type = self.parse_type()?;
             let param_name = self.expect_identifier().ok_or("Expected parameter name")?;
             parameters.push(Parameter {
                 name: param_name,
                 r#type: param_type,
                 line: self.peek().line(),
             });

             if !self.expect_operator(",") {
                 break;
             }
         }
         if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
             let _ = writeln!(log, "[PARSER] Parsed {} parameters", parameters.len());
         }

         if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
             let _ = writeln!(log, "[PARSER] Expecting ')'...");
         }
         if !self.expect_operator(")") {
             if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
                 let _ = writeln!(log, "[PARSER] ERROR: No ')' found");
             }
             return Err("Expected ')' after parameters".to_string());
         }
         if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
             let _ = writeln!(log, "[PARSER] Found ')'");
         }

         let mut body = Vec::new();
         if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
             let _ = writeln!(log, "[PARSER] Looking for function body or semicolon, current token: {:?}", self.peek());
         }
         let is_definition = if self.expect_operator("{") {
             if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
                 let _ = writeln!(log, "[PARSER] Found '{{', parsing function body...");
             }
             while !self.peek_operator("}") && !self.is_at_end() {
                 if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
                     let _ = writeln!(log, "[PARSER] Parsing statement in body at pos {}", self.position);
                 }
                 body.push(self.parse_statement()?);
             }
             if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
                 let _ = writeln!(log, "[PARSER] Parsed {} statements in body", body.len());
             }
             if !self.expect_operator("}") {
                 if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
                     let _ = writeln!(log, "[PARSER] ERROR: No '}}' found");
                 }
                 return Err("Expected '}' to close function".to_string());
             }
             if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
                 let _ = writeln!(log, "[PARSER] Found '}}'");
             }
             true
         } else if self.expect_operator(";") {
             if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
                 let _ = writeln!(log, "[PARSER] Found ';' - function declaration only");
             }
             false
         } else {
             if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
                 let _ = writeln!(log, "[PARSER] ERROR: Expected '{{' or ';' after function signature, got: {:?}", self.peek());
             }
             return Err("Expected ';' or '{' after function signature".to_string());
         };

         if let Ok(mut log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_debug.log") {
             let _ = writeln!(log, "[PARSER] parse_function_declaration completed successfully: {}", name);
         }
         Ok(Item::Function {
             return_type,
             name,
             parameters,
             body,
             is_definition,
             line,
             is_member: false,
         })
      }

      fn parse_global_variable_declaration(&mut self) -> Result<Item, String> {
          let line = self.peek().line();
          let var_type = self.parse_type()?;
          let var_name = self.expect_identifier().ok_or("Expected variable name")?;
          
          if !self.expect_operator(";") {
              return Err("Expected ';' after variable declaration".to_string());
          }
          
          Ok(Item::GlobalVariable {
              r#type: var_type,
              name: var_name,
              line,
          })
      }

      fn parse_type(&mut self) -> Result<String, String> {
         let mut type_str = String::new();

         // Handle const keyword
         if self.expect_keyword("const") {
             type_str.push_str("const ");
         }

         // Main type
         if let Token::Keyword { value, .. } = self.peek() {
             if TYPE_KEYWORDS.contains(&value.as_str()) || value == "class" || value == "struct"
                 || self.peek_identifier()
             {
                 type_str.push_str(value);
                 self.advance();
             } else {
                 return Err("Expected type keyword".to_string());
             }
         } else if self.peek_identifier() {
             if let Some(id) = self.expect_identifier() {
                 type_str.push_str(&id);
             }
         } else {
             return Err("Expected type".to_string());
         }

         // Handle reference, pointer, array, etc.
         let mut saw_modification = true;
         let mut iterations = 0;
         while saw_modification && iterations < 20 {  // Safety limit
             iterations += 1;
             saw_modification = false;
             
             if self.expect_operator("&") {
                 type_str.push('&');
                 saw_modification = true;
             } else if self.expect_operator("*") {
                 type_str.push('*');
                 saw_modification = true;
             } else if self.expect_operator("[") {
                 type_str.push('[');
                 saw_modification = true;
                 while !self.peek_operator("]") && !self.is_at_end() {
                     if let Token::Number { value, .. } = self.peek() {
                         type_str.push_str(value);
                         self.advance();
                     } else {
                         break;
                     }
                 }
                 if self.expect_operator("]") {
                     type_str.push(']');
                 }
             } else if self.peek_operator("<") {
                 // Template parameters
                 type_str.push('<');
                 self.advance();
                 saw_modification = true;
                 let mut depth = 1;
                while depth > 0 && !self.is_at_end() {
                    if self.peek_operator("<") {
                        type_str.push('<');
                        depth += 1;
                        self.advance();
                    } else if self.peek_operator(">") {
                        type_str.push('>');
                        depth -= 1;
                        self.advance();
                    } else if let Token::Identifier { value, .. } = self.peek() {
                        type_str.push_str(value);
                        self.advance();
                    } else if self.peek_operator(",") {
                        type_str.push(',');
                        self.advance();
                    } else {
                        self.advance();
                    }
                }
            }
        }

        Ok(type_str)
    }

     fn parse_statement(&mut self) -> Result<Statement, String> {
         if self.expect_keyword("if") {
             self.parse_if_statement()
         } else if self.expect_keyword("while") {
             self.parse_while_statement()
         } else if self.peek_keyword("for") {
             let for_column = self.peek().column();
             self.expect_keyword("for");
             self.parse_for_statement(for_column)
         } else if self.expect_keyword("return") {
             self.parse_return_statement()
         } else if self.expect_keyword("break") {
             self.expect_operator(";");
             Ok(Statement::Break)
         } else if self.expect_keyword("continue") {
             self.expect_operator(";");
             Ok(Statement::Continue)
         } else if self.expect_keyword("switch") {
             self.parse_switch_statement()
         } else if self.expect_operator("{") {
             self.parse_block_statement()
         } else {
             // Variable declaration or expression statement
             let saved_pos = self.position;

             // Try to parse as variable declaration
             if let Ok(stmt) = self.try_parse_variable_declaration() {
                 return Ok(stmt);
             }

             // Reset and try as expression statement
             self.position = saved_pos;
             let expr = self.parse_expression()?;
             if self.expect_operator(";") {
                 Ok(Statement::Expression(Box::new(expr)))
             } else {
                 Err("Expected ';' after expression".to_string())
             }
         }
     }

    fn try_parse_variable_declaration(&mut self) -> Result<Statement, String> {
        let r#type = self.parse_type()?;
        let name = self.expect_identifier().ok_or("Expected variable name")?;
        let line = self.peek().line();

        let initializer = if self.expect_operator("=") {
            Some(Box::new(self.parse_expression()?))
        } else if self.expect_operator("[") {
            // Array initialization
            let mut expr_str = String::from("[");
            while !self.peek_operator("]") && !self.is_at_end() {
                if let Token::Number { value, .. } = self.peek() {
                    expr_str.push_str(value);
                    self.advance();
                } else {
                    break;
                }
            }
            if self.expect_operator("]") {
                expr_str.push(']');
                if self.expect_operator("=") && self.expect_operator("{") {
                    // Array initializer
                    while !self.peek_operator("}") && !self.is_at_end() {
                        self.advance();
                    }
                    if self.expect_operator("}") {
                        // Successfully parsed array initializer
                    }
                }
            }
            None
        } else {
            None
        };

        if !self.expect_operator(";") {
            return Err("Expected ';' after variable declaration".to_string());
        }

        Ok(Statement::VariableDecl {
            name,
            r#type,
            initializer,
            line,
        })
    }

    fn parse_if_statement(&mut self) -> Result<Statement, String> {
        if !self.expect_operator("(") {
            return Err("Expected '(' after 'if'".to_string());
        }
        let condition = Box::new(self.parse_expression()?);
        if !self.expect_operator(")") {
            return Err("Expected ')' after if condition".to_string());
        }

        let body = if self.peek_operator("{") {
            self.parse_block_statement()
                .map(|stmt| match stmt {
                    Statement::Block { statements } => statements,
                    other => vec![other],
                })?
        } else {
            vec![self.parse_statement()?]
        };

        let else_body = if self.expect_keyword("else") {
            Some(
                if self.peek_operator("{") {
                    self.parse_block_statement()
                        .map(|stmt| match stmt {
                            Statement::Block { statements } => statements,
                            other => vec![other],
                        })?
                } else {
                    vec![self.parse_statement()?]
                },
            )
        } else {
            None
        };

        Ok(Statement::If {
            condition,
            body,
            else_body,
        })
    }

    fn parse_while_statement(&mut self) -> Result<Statement, String> {
        if !self.expect_operator("(") {
            return Err("Expected '(' after 'while'".to_string());
        }
        let condition = Box::new(self.parse_expression()?);
        if !self.expect_operator(")") {
            return Err("Expected ')' after while condition".to_string());
        }

        let body = if self.peek_operator("{") {
            self.parse_block_statement()
                .map(|stmt| match stmt {
                    Statement::Block { statements } => statements,
                    other => vec![other],
                })?
        } else {
            vec![self.parse_statement()?]
        };

        Ok(Statement::While { condition, body })
    }

    fn parse_for_statement(&mut self, for_column: usize) -> Result<Statement, String> {
        let line = self.peek().line();
        if !self.expect_operator("(") {
            return Err("Expected '(' after 'for'".to_string());
        }

        // Check if range-based or traditional
        let for_type = self.detect_for_loop_type();

        if for_type == ForLoopType::Range {
            self.parse_range_for_statement()
        } else {
            // Traditional for loop - record violation
            self.parse_traditional_for_statement(line, for_column)
        }
    }

    fn detect_for_loop_type(&self) -> ForLoopType {
        let _saved = self.position;

        // Look for pattern: [const] <type> <identifier> ':' vs '=' or ';'
        let mut pos = self.position;

        // Skip 'const' if present
        if pos < self.tokens.len() {
            if let Token::Keyword { value, .. } = &self.tokens[pos] {
                if value == "const" {
                    pos += 1;
                }
            }
        }

        // Check for type keyword or 'auto'
        if pos < self.tokens.len() {
            if let Token::Keyword { value, .. } = &self.tokens[pos] {
                if TYPE_KEYWORDS.contains(&value.as_str()) || value == "auto" {
                    pos += 1;

                    // Skip reference
                    if pos < self.tokens.len() {
                        if let Token::Operator { value, .. } = &self.tokens[pos] {
                            if value == "&" || value == "&&" {
                                pos += 1;
                            }
                        }
                    }

                    // Check for identifier
                    if pos < self.tokens.len() {
                        if matches!(&self.tokens[pos], Token::Identifier { .. }) {
                            pos += 1;

                            // Look at next token
                            if pos < self.tokens.len() {
                                match &self.tokens[pos] {
                                    Token::Operator { value, .. } if value == ":" => {
                                        return ForLoopType::Range;
                                    }
                                    Token::Operator { value, .. } if value == "=" || value == ";" => {
                                        return ForLoopType::Traditional;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }

        ForLoopType::Range
    }

    fn parse_range_for_statement(&mut self) -> Result<Statement, String> {
        let line = self.peek().line();
        let const_kw = self.expect_keyword("const");

        let var_type = if const_kw {
            "const auto".to_string()
        } else {
            "auto".to_string()
        };

        let variable = self.expect_identifier().ok_or("Expected variable name")?;

        let _ref = self.expect_operator("&");

        if !self.expect_operator(":") {
            return Err("Expected ':' in range-based for".to_string());
        }

        let iterable = Box::new(self.parse_expression()?);

        if !self.expect_operator(")") {
            return Err("Expected ')' after for header".to_string());
        }

        let body = if self.peek_operator("{") {
            self.parse_block_statement()
                .map(|stmt| match stmt {
                    Statement::Block { statements } => statements,
                    other => vec![other],
                })?
        } else {
            vec![self.parse_statement()?]
        };

        Ok(Statement::For {
            is_range: true,
            variable,
            var_type,
            iterable,
            body,
            line,
        })
    }

    fn parse_traditional_for_statement(&mut self, line: usize, for_column: usize) -> Result<Statement, String> {
        // Record violation for traditional for loop (if enabled in config)
        if self.config.rules.traditional_for_loops {
            // Convert 1-indexed column to 0-indexed character position
            let start_char = for_column.saturating_sub(1);
            self.violations.push(ConstraintViolation {
                violation_type: "traditional_for_loop".to_string(),
                line,
                start_char,
                end_char: start_char + 3,
                message: "Traditional for loops not supported - use range-based for (for (auto& item : container)) instead".to_string(),
            });
        }

        // Skip to the end of the for loop parentheses
        let mut depth = 1;
        while depth > 0 && !self.is_at_end() {
            if self.expect_operator("(") {
                depth += 1;
            } else if self.expect_operator(")") {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            } else {
                self.advance();
            }
        }

        // Parse the body
        let body = if self.peek_operator("{") {
            self.parse_block_statement()
                .map(|stmt| match stmt {
                    Statement::Block { statements } => statements,
                    other => vec![other],
                })?
        } else {
            vec![self.parse_statement()?]
        };

        Ok(Statement::For {
            is_range: false,
            variable: "_i".to_string(),
            var_type: "int".to_string(),
            iterable: Box::new(Expression::NumberLiteral {
                value: "0".to_string(),
                line,
            }),
            body,
            line,
        })
    }

    fn parse_switch_statement(&mut self) -> Result<Statement, String> {
        if !self.expect_operator("(") {
            return Err("Expected '(' after 'switch'".to_string());
        }
        let _expr = self.parse_expression()?;
        if !self.expect_operator(")") {
            return Err("Expected ')' after switch expression".to_string());
        }

        if !self.expect_operator("{") {
            return Err("Expected '{' after switch".to_string());
        }

        let mut statements = Vec::new();
        while !self.peek_operator("}") && !self.is_at_end() {
            if self.expect_keyword("case") {
                // Parse case label
                self.advance(); // skip number
                if self.expect_operator(":") {
                    // Parse case statements
                    while !self.peek_keyword("case") && !self.peek_keyword("default")
                        && !self.peek_operator("}")
                        && !self.is_at_end()
                    {
                        statements.push(self.parse_statement()?);
                    }
                }
            } else if self.expect_keyword("default") {
                if self.expect_operator(":") {
                    while !self.peek_keyword("case") && !self.peek_keyword("default")
                        && !self.peek_operator("}")
                        && !self.is_at_end()
                    {
                        statements.push(self.parse_statement()?);
                    }
                }
            } else {
                self.advance();
            }
        }

        if !self.expect_operator("}") {
            return Err("Expected '}' to close switch".to_string());
        }

        Ok(Statement::Block { statements })
    }

    fn parse_block_statement(&mut self) -> Result<Statement, String> {
        let mut statements = Vec::new();
        while !self.peek_operator("}") && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }
        if !self.expect_operator("}") {
            return Err("Expected '}' to close block".to_string());
        }
        Ok(Statement::Block { statements })
    }

    fn parse_return_statement(&mut self) -> Result<Statement, String> {
        let expression = if self.peek_operator(";") {
            None
        } else {
            Some(Box::new(self.parse_expression()?))
        };
        if !self.expect_operator(";") {
            return Err("Expected ';' after return".to_string());
        }
        Ok(Statement::Return { expression })
    }

    fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_ternary()
    }

    fn parse_ternary(&mut self) -> Result<Expression, String> {
        let expr = self.parse_logical_or()?;

        if self.expect_operator("?") {
            let true_expr = Box::new(self.parse_expression()?);
            if !self.expect_operator(":") {
                return Err("Expected ':' in ternary operator".to_string());
            }
            let false_expr = Box::new(self.parse_expression()?);
            let line = expr.line();
            let result = Expression::TernaryOp {
                condition: Box::new(expr),
                true_expr,
                false_expr,
                line,
            };
            Ok(result)
        } else {
            Ok(expr)
        }
    }

    fn parse_logical_or(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_logical_and()?;

        while self.peek_operator("||") {
            let operator = if let Token::Operator { value, .. } = self.advance() {
                value
            } else {
                "||".to_string()
            };
            let right = self.parse_logical_and()?;
            let line = left.line();
            left = Expression::BinaryOp {
                operator,
                left: Box::new(left),
                right: Box::new(right),
                line,
            };
        }

        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_bitwise_or()?;

        while self.peek_operator("&&") {
            let operator = if let Token::Operator { value, .. } = self.advance() {
                value
            } else {
                "&&".to_string()
            };
            let right = self.parse_bitwise_or()?;
            let line = left.line();
            left = Expression::BinaryOp {
                operator,
                left: Box::new(left),
                right: Box::new(right),
                line,
            };
        }

        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_comparison()?;

        while let Token::Operator { value, .. } = self.peek() {
            if value == "==" || value == "!=" {
                let operator = value.clone();
                self.advance();
                let right = self.parse_comparison()?;
                let line = left.line();
                left = Expression::BinaryOp {
                    operator,
                    left: Box::new(left),
                    right: Box::new(right),
                    line,
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_shift()?;

        while let Token::Operator { value, .. } = self.peek() {
            if value == "<" || value == ">" || value == "<=" || value == ">=" {
                let operator = value.clone();
                self.advance();
                let right = self.parse_shift()?;
                let line = left.line();
                left = Expression::BinaryOp {
                    operator,
                    left: Box::new(left),
                    right: Box::new(right),
                    line,
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_additive()?;

        while let Token::Operator { value, .. } = self.peek() {
            if value == "<<" || value == ">>" {
                let operator = value.clone();
                self.advance();
                let right = self.parse_additive()?;
                let line = left.line();
                left = Expression::BinaryOp {
                    operator,
                    left: Box::new(left),
                    right: Box::new(right),
                    line,
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_bitwise_or(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_bitwise_xor()?;

        while self.peek_operator("|") {
            let operator = "|".to_string();
            self.advance();
            let right = self.parse_bitwise_xor()?;
            let line = left.line();
            left = Expression::BinaryOp {
                operator,
                left: Box::new(left),
                right: Box::new(right),
                line,
            };
        }

        Ok(left)
    }

    fn parse_bitwise_xor(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_bitwise_and()?;

        while self.peek_operator("^") {
            let operator = "^".to_string();
            self.advance();
            let right = self.parse_bitwise_and()?;
            let line = left.line();
            left = Expression::BinaryOp {
                operator,
                left: Box::new(left),
                right: Box::new(right),
                line,
            };
        }

        Ok(left)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_equality()?;

        while self.peek_operator("&") && !self.peek_operator("&&") {
            let operator = "&".to_string();
            self.advance();
            let right = self.parse_equality()?;
            let line = left.line();
            left = Expression::BinaryOp {
                operator,
                left: Box::new(left),
                right: Box::new(right),
                line,
            };
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_multiplicative()?;

        while let Token::Operator { value, .. } = self.peek() {
            if value == "+" || value == "-" {
                let operator = value.clone();
                self.advance();
                let right = self.parse_multiplicative()?;
                let line = left.line();
                left = Expression::BinaryOp {
                    operator,
                    left: Box::new(left),
                    right: Box::new(right),
                    line,
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_unary()?;

        while let Token::Operator { value, .. } = self.peek() {
            if value == "*" || value == "/" || value == "%" {
                let operator = value.clone();
                self.advance();
                let right = self.parse_unary()?;
                let line = left.line();
                left = Expression::BinaryOp {
                    operator,
                    left: Box::new(left),
                    right: Box::new(right),
                    line,
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expression, String> {
        let line = self.peek().line();

        if let Token::Operator { value, .. } = self.peek() {
            if value == "!" || value == "-" || value == "+" || value == "~" {
                let operator = value.clone();
                self.advance();
                let operand = Box::new(self.parse_unary()?);
                return Ok(Expression::UnaryOp {
                    operator,
                    operand,
                    line,
                });
            }
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expression, String> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek() {
                Token::Operator { value, .. } if value == "++" || value == "--" => {
                    let operator = value.clone();
                    let line = expr.line();
                    self.advance();
                    expr = Expression::UnaryOp {
                        operator,
                        operand: Box::new(expr),
                        line,
                    };
                }
                Token::Operator { value, .. } if value == "[" => {
                    self.advance();
                    let index = Box::new(self.parse_expression()?);
                    if !self.expect_operator("]") {
                        return Err("Expected ']'".to_string());
                    }
                    let line = expr.line();
                    expr = Expression::ArrayAccess {
                        base: Box::new(expr),
                        index,
                        line,
                    };
                }
                Token::Operator { value, .. } if value == "." => {
                    self.advance();
                    let field = self.expect_identifier().ok_or("Expected field name")?;
                    let line = expr.line();
                    expr = Expression::MemberAccess {
                        object: Box::new(expr),
                        field,
                        line,
                    };
                }
                Token::Operator { value, .. } if value == "->" => {
                    self.advance();
                    let field = self.expect_identifier().ok_or("Expected field name")?;
                    let line = expr.line();
                    expr = Expression::PointerMemberAccess {
                        object: Box::new(expr),
                        field,
                        line,
                    };
                }
                Token::Operator { value, .. } if value == "(" => {
                    self.advance();
                    let mut args = Vec::new();
                    while !self.peek_operator(")") && !self.is_at_end() {
                        args.push(self.parse_expression()?);
                        if !self.expect_operator(",") {
                            break;
                        }
                    }
                    if !self.expect_operator(")") {
                        return Err("Expected ')'".to_string());
                    }
                    let line = expr.line();

                    // Determine if function call or method call
                    expr = match expr {
                        Expression::VariableRef { name, .. } => Expression::FunctionCall {
                            name,
                            args,
                            line,
                        },
                        _ => Expression::MethodCall {
                            object: Box::new(expr),
                            method: "call".to_string(),
                            args,
                            line,
                        },
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expression, String> {
        let line = self.peek().line();

        match self.peek() {
            Token::Number { value, .. } => {
                let num = value.clone();
                self.advance();
                Ok(Expression::NumberLiteral {
                    value: num,
                    line,
                })
            }
            Token::String { value, .. } => {
                let str_val = value.clone();
                self.advance();
                Ok(Expression::StringLiteral {
                    value: str_val,
                    line,
                })
            }
            Token::Char { value, .. } => {
                let char_val = value.clone();
                self.advance();
                Ok(Expression::CharLiteral {
                    value: char_val,
                    line,
                })
            }
            Token::Keyword { value, .. } if value == "true" || value == "false" => {
                let bool_val = value == "true";
                self.advance();
                Ok(Expression::BoolLiteral {
                    value: bool_val,
                    line,
                })
            }
            Token::Identifier { value, .. } => {
                let id = value.clone();
                self.advance();
                Ok(Expression::VariableRef { name: id, line })
            }
            Token::Operator { value, .. } if value == "(" => {
                self.advance();
                let expr = Box::new(self.parse_expression()?);
                if !self.expect_operator(")") {
                    return Err("Expected ')' after expression".to_string());
                }
                Ok(Expression::Parenthesized { expr, line })
            }
            _ => Err(format!("Unexpected token: {:?}", self.peek())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::Tokenizer;

    fn parse(source: &str) -> Result<SourceFile, String> {
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer.tokenize().map_err(|e| e.to_string())?;
        let mut parser = Parser::new(tokens);
        Ok(parser.parse())
    }

    #[test]
    fn test_parse_simple_function() {
        let source = "int main() { return 0; }";
        let ast = parse(source).unwrap();

        assert_eq!(ast.items.len(), 1);
        assert!(matches!(&ast.items[0], Item::Function { .. }));
    }

    #[test]
    fn test_detect_forbidden_keyword() {
        let source = "virtual void test() {}";
        let ast = parse(source).unwrap();

        assert_eq!(ast.forbidden_keywords.len(), 1);
        assert_eq!(ast.forbidden_keywords[0].keyword, "virtual");
    }

    #[test]
    fn test_detect_preprocessor() {
        let source = "#include <iostream>\nint main() {}";
        let ast = parse(source).unwrap();

        assert!(ast
            .constraint_violations
            .iter()
            .any(|v| v.violation_type == "preprocessor_directive"));
    }
}
