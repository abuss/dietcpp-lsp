/// Code actions and semantic tokens for DietC++ LSP
use lsp_types::*;
use crate::ast::*;

/// Generate semantic tokens for syntax highlighting
pub fn generate_semantic_tokens(_ast: &SourceFile) -> SemanticTokensResult {
    // For now, return empty semantic tokens
    // In a full implementation, this would walk the AST and generate proper semantic tokens
    SemanticTokensResult::Tokens(SemanticTokens {
        result_id: None,
        data: vec![],
    })
}

/// Generate code actions for violations
pub fn generate_code_actions(
    violations: &[ConstraintViolation],
    _uri: &str,
) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();

    for violation in violations {
        match violation.violation_type.as_str() {
            "traditional_for_loop" => {
                // Suggest converting to range-based for loop
                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Convert to range-based for loop".to_string(),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![Diagnostic {
                        range: Range {
                            start: Position {
                                line: (violation.line as u32).saturating_sub(1),
                                character: violation.start_char as u32,
                            },
                            end: Position {
                                line: (violation.line as u32).saturating_sub(1),
                                character: violation.end_char as u32,
                            },
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: Some(NumberOrString::String("traditional_for_loop".to_string())),
                        source: Some("DietC++".to_string()),
                        message: violation.message.clone(),
                        related_information: None,
                        tags: None,
                        code_description: None,
                        data: None,
                    }]),
                    edit: Some(WorkspaceEdit {
                        changes: None,
                        document_changes: None,
                        change_annotations: None,
                    }),
                    command: None,
                    is_preferred: Some(true),
                    disabled: None,
                    data: None,
                }));
            }
            "raw_pointer_usage" => {
                // Suggest using smart pointers or containers
                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Replace with std::unique_ptr".to_string(),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![Diagnostic {
                        range: Range {
                            start: Position {
                                line: (violation.line as u32).saturating_sub(1),
                                character: violation.start_char as u32,
                            },
                            end: Position {
                                line: (violation.line as u32).saturating_sub(1),
                                character: violation.end_char as u32,
                            },
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: Some(NumberOrString::String("raw_pointer_usage".to_string())),
                        source: Some("DietC++".to_string()),
                        message: violation.message.clone(),
                        related_information: None,
                        tags: None,
                        code_description: None,
                        data: None,
                     }]),
                    edit: Some(WorkspaceEdit {
                        changes: None,
                        document_changes: None,
                        change_annotations: None,
                     }),
                    command: None,
                    is_preferred: Some(true),
                    disabled: None,
                    data: None,
                }));

                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Replace with std::shared_ptr".to_string(),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![Diagnostic {
                        range: Range {
                            start: Position {
                                line: (violation.line as u32).saturating_sub(1),
                                character: violation.start_char as u32,
                            },
                            end: Position {
                                line: (violation.line as u32).saturating_sub(1),
                                character: violation.end_char as u32,
                            },
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: Some(NumberOrString::String("raw_pointer_usage".to_string())),
                        source: Some("DietC++".to_string()),
                        message: violation.message.clone(),
                        related_information: None,
                        tags: None,
                        code_description: None,
                        data: None,
                    }]),
                    edit: Some(WorkspaceEdit {
                        changes: None,
                        document_changes: None,
                        change_annotations: None,
                    }),
                    command: None,
                    is_preferred: Some(false),
                    disabled: None,
                    data: None,
                }));
            }
            "preprocessor_directive" => {
                // Suggest alternatives
                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Learn about preprocessor alternative".to_string(),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![Diagnostic {
                        range: Range {
                            start: Position {
                                line: (violation.line as u32).saturating_sub(1),
                                character: violation.start_char as u32,
                            },
                            end: Position {
                                line: (violation.line as u32).saturating_sub(1),
                                character: violation.end_char as u32,
                            },
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: Some(NumberOrString::String("preprocessor_directive".to_string())),
                        source: Some("DietC++".to_string()),
                        message: violation.message.clone(),
                        related_information: None,
                        tags: None,
                        code_description: None,
                        data: None,
                    }]),
                    edit: Some(WorkspaceEdit {
                        changes: None,
                        document_changes: None,
                        change_annotations: None,
                    }),
                    command: None,
                    is_preferred: Some(true),
                    disabled: None,
                    data: None,
                }));
            }
            _ => {}
        }
    }

    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_tokens_generation() {
        let ast = SourceFile::new();
        let tokens = generate_semantic_tokens(&ast);
        match tokens {
            SemanticTokensResult::Tokens(t) => {
                assert_eq!(t.data.len(), 0);
            }
            _ => panic!("Expected SemanticTokens"),
        }
    }

    #[test]
    fn test_code_actions_for_traditional_for_loop() {
        let violation = ConstraintViolation {
            violation_type: "traditional_for_loop".to_string(),
            line: 5,
            start_char: 0,
            end_char: 3,
            message: "Traditional for loop not supported".to_string(),
        };

        let actions = generate_code_actions(&[violation], "file:///test.cpp");
        assert!(!actions.is_empty());
    }

    #[test]
    fn test_code_actions_for_raw_pointer() {
        let violation = ConstraintViolation {
            violation_type: "raw_pointer_usage".to_string(),
            line: 3,
            start_char: 0,
            end_char: 5,
            message: "Raw pointer not allowed".to_string(),
        };

        let actions = generate_code_actions(&[violation], "file:///test.cpp");
        assert!(actions.len() >= 2); // unique_ptr and shared_ptr suggestions
    }
}
