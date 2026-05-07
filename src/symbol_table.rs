/// Symbol table for scope-aware symbol management
use crate::ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
    pub scope: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Function,
    Variable,
    Class,
    Struct,
    Enum,
    Namespace,
    Parameter,
}

#[derive(Debug)]
struct Scope {
    name: String,
    symbols: HashMap<String, Vec<Symbol>>,
    parent_idx: Option<usize>,
}

pub struct SymbolTable {
    scopes: Vec<Scope>,
    current_scope_idx: usize,
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut scopes = Vec::new();
        scopes.push(Scope {
            name: "global".to_string(),
            symbols: HashMap::new(),
            parent_idx: None,
        });

        SymbolTable {
            scopes,
            current_scope_idx: 0,
        }
    }

    pub fn enter_scope(&mut self, name: String) {
        let parent_idx = self.current_scope_idx;
        self.scopes.push(Scope {
            name,
            symbols: HashMap::new(),
            parent_idx: Some(parent_idx),
        });
        self.current_scope_idx = self.scopes.len() - 1;
    }

    pub fn exit_scope(&mut self) {
        if let Some(parent_idx) = self.scopes[self.current_scope_idx].parent_idx {
            self.current_scope_idx = parent_idx;
        }
    }

    pub fn define(&mut self, symbol: Symbol) {
        if let Some(scope) = self.scopes.get_mut(self.current_scope_idx) {
            scope
                .symbols
                .entry(symbol.name.clone())
                .or_insert_with(Vec::new)
                .push(symbol);
        }
    }

    pub fn lookup(&self, name: &str) -> Option<Symbol> {
        // Search from current scope upwards
        let mut idx = self.current_scope_idx;
        loop {
            if let Some(scope) = self.scopes.get(idx) {
                if let Some(symbols) = scope.symbols.get(name) {
                    if let Some(symbol) = symbols.last() {
                        return Some(symbol.clone());
                    }
                }

                if let Some(parent_idx) = scope.parent_idx {
                    idx = parent_idx;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        None
    }

    pub fn find_references(&self, name: &str) -> Vec<Symbol> {
        let mut results = Vec::new();

        for scope in &self.scopes {
            if let Some(symbols) = scope.symbols.get(name) {
                results.extend(symbols.clone());
            }
        }

        results
    }

    pub fn build_from_ast(&mut self, ast: &SourceFile) {
        for item in &ast.items {
            self.visit_item(item);
        }
    }

    fn visit_item(&mut self, item: &Item) {
        match item {
            Item::Function {
                name,
                parameters,
                body,
                line,
                ..
            } => {
                // Define function symbol
                self.define(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Function,
                    line: *line,
                    scope: self.current_scope_name(),
                });

                // Enter function scope
                self.enter_scope(name.clone());

                // Define parameters
                for param in parameters {
                    self.define(Symbol {
                        name: param.name.clone(),
                        kind: SymbolKind::Parameter,
                        line: param.line,
                        scope: self.current_scope_name(),
                    });
                }

                // Visit body
                for stmt in body {
                    self.visit_statement(stmt);
                }

                // Exit function scope
                self.exit_scope();
            }
            Item::Class {
                name,
                members,
                line,
                ..
            }
            | Item::Struct {
                name,
                members,
                line,
                ..
            } => {
                let kind = if matches!(item, Item::Class { .. }) {
                    SymbolKind::Class
                } else {
                    SymbolKind::Struct
                };

                self.define(Symbol {
                    name: name.clone(),
                    kind,
                    line: *line,
                    scope: self.current_scope_name(),
                });

                self.enter_scope(name.clone());

                for member in members {
                    match member {
                        ClassMember::Function {
                            name: method_name,
                            parameters,
                            ..
                        } => {
                            self.define(Symbol {
                                name: method_name.clone(),
                                kind: SymbolKind::Function,
                                line: *line,
                                scope: self.current_scope_name(),
                            });

                            for param in parameters {
                                self.define(Symbol {
                                    name: param.name.clone(),
                                    kind: SymbolKind::Parameter,
                                    line: param.line,
                                    scope: self.current_scope_name(),
                                });
                            }
                        }
                        ClassMember::Variable {
                            name: var_name,
                            line: var_line,
                            ..
                        } => {
                            self.define(Symbol {
                                name: var_name.clone(),
                                kind: SymbolKind::Variable,
                                line: *var_line,
                                scope: self.current_scope_name(),
                            });
                        }
                    }
                }

                self.exit_scope();
            }
            Item::Enum { name, members, line } => {
                self.define(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Enum,
                    line: *line,
                    scope: self.current_scope_name(),
                });

                self.enter_scope(name.clone());

                for member in members {
                    self.define(Symbol {
                        name: member.name.clone(),
                        kind: SymbolKind::Variable,
                        line: member.line,
                        scope: self.current_scope_name(),
                    });
                }

                self.exit_scope();
            }
            Item::Namespace { name, items, line } => {
                self.define(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Namespace,
                    line: *line,
                    scope: self.current_scope_name(),
                });

                self.enter_scope(name.clone());

                for nested_item in items {
                    self.visit_item(nested_item);
                }

                self.exit_scope();
            }
            Item::GlobalVariable { name, line, .. } => {
                self.define(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Variable,
                    line: *line,
                    scope: self.current_scope_name(),
                });
            }
            Item::UsingDeclaration { .. } => {
                // Using declarations don't create symbols
            }
        }
    }

    fn visit_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDecl {
                name,
                line,
                initializer,
                ..
            } => {
                self.define(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Variable,
                    line: *line,
                    scope: self.current_scope_name(),
                });

                if let Some(expr) = initializer {
                    self.visit_expression(expr);
                }
            }
            Statement::Block { statements } => {
                for s in statements {
                    self.visit_statement(s);
                }
            }
            Statement::If {
                body,
                else_body,
                ..
            } => {
                for s in body {
                    self.visit_statement(s);
                }
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        self.visit_statement(s);
                    }
                }
            }
            Statement::While { body, .. } => {
                for s in body {
                    self.visit_statement(s);
                }
            }
            Statement::For { body, .. } => {
                for s in body {
                    self.visit_statement(s);
                }
            }
            _ => {}
        }
    }

    fn visit_expression(&mut self, _expr: &Expression) {
        // Could track variable references here
    }

    fn current_scope_name(&self) -> String {
        if let Some(scope) = self.scopes.get(self.current_scope_idx) {
            scope.name.clone()
        } else {
            "unknown".to_string()
        }
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_table_creation() {
        let table = SymbolTable::new();
        assert_eq!(table.scopes.len(), 1);
        assert_eq!(table.current_scope_idx, 0);
    }

    #[test]
    fn test_enter_exit_scope() {
        let mut table = SymbolTable::new();
        assert_eq!(table.current_scope_idx, 0);

        table.enter_scope("function1".to_string());
        assert_eq!(table.current_scope_idx, 1);
        assert_eq!(table.scopes.len(), 2);

        table.exit_scope();
        assert_eq!(table.current_scope_idx, 0);
    }

    #[test]
    fn test_define_and_lookup() {
        let mut table = SymbolTable::new();

        let symbol = Symbol {
            name: "x".to_string(),
            kind: SymbolKind::Variable,
            line: 1,
            scope: "global".to_string(),
        };

        table.define(symbol.clone());

        let found = table.lookup("x");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "x");
    }

    #[test]
    fn test_scope_shadowing() {
        let mut table = SymbolTable::new();

        // Define in global scope
        table.define(Symbol {
            name: "x".to_string(),
            kind: SymbolKind::Variable,
            line: 1,
            scope: "global".to_string(),
        });

        // Enter local scope
        table.enter_scope("func".to_string());

        // Define same name in local scope
        table.define(Symbol {
            name: "x".to_string(),
            kind: SymbolKind::Variable,
            line: 2,
            scope: "func".to_string(),
        });

        // Lookup should return the local one (most recent)
        let found = table.lookup("x").unwrap();
        assert_eq!(found.line, 2);

        // Exit scope
        table.exit_scope();

        // Now should find global one
        let found = table.lookup("x").unwrap();
        assert_eq!(found.line, 1);
    }

    #[test]
    fn test_find_references() {
        let mut table = SymbolTable::new();

        table.define(Symbol {
            name: "global_var".to_string(),
            kind: SymbolKind::Variable,
            line: 1,
            scope: "global".to_string(),
        });

        table.enter_scope("func1".to_string());
        table.define(Symbol {
            name: "local_var".to_string(),
            kind: SymbolKind::Variable,
            line: 2,
            scope: "func1".to_string(),
        });
        table.exit_scope();

        let references = table.find_references("global_var");
        assert_eq!(references.len(), 1);

        let references = table.find_references("local_var");
        assert_eq!(references.len(), 1);
    }
}
