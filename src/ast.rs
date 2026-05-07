/// Abstract Syntax Tree node definitions for DietC++

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub items: Vec<Item>,
    pub forbidden_keywords: Vec<ForbiddenKeyword>,
    pub constraint_violations: Vec<ConstraintViolation>,
    pub parse_errors: Vec<ParseError>,
}

impl SourceFile {
    pub fn new() -> Self {
        SourceFile {
            items: Vec::new(),
            forbidden_keywords: Vec::new(),
            constraint_violations: Vec::new(),
            parse_errors: Vec::new(),
        }
    }
}

impl Default for SourceFile {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum Item {
    UsingDeclaration { namespace: String, line: usize },
    Namespace { name: String, items: Vec<Item>, line: usize },
    Class { name: String, members: Vec<ClassMember>, line: usize },
    Struct { name: String, members: Vec<ClassMember>, line: usize },
    Enum { name: String, members: Vec<EnumMember>, line: usize },
    Function {
        return_type: String,
        name: String,
        parameters: Vec<Parameter>,
        body: Vec<Statement>,
        is_definition: bool,
        line: usize,
        is_member: bool,
    },
    GlobalVariable {
        r#type: String,
        name: String,
        line: usize,
    },
}

#[derive(Debug, Clone)]
pub enum ClassMember {
    Function {
        visibility: String,
        return_type: String,
        name: String,
        parameters: Vec<Parameter>,
        body: Vec<Statement>,
        line: usize,
    },
    Variable {
        visibility: String,
        r#type: String,
        name: String,
        line: usize,
    },
}

#[derive(Debug, Clone)]
pub struct EnumMember {
    pub name: String,
    pub value: Option<String>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub r#type: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Block {
        statements: Vec<Statement>,
    },
    If {
        condition: Box<Expression>,
        body: Vec<Statement>,
        else_body: Option<Vec<Statement>>,
    },
    While {
        condition: Box<Expression>,
        body: Vec<Statement>,
    },
    For {
        is_range: bool,
        variable: String,
        var_type: String,
        iterable: Box<Expression>,
        body: Vec<Statement>,
        line: usize,
    },
    Return {
        expression: Option<Box<Expression>>,
    },
    Break,
    Continue,
    Expression(Box<Expression>),
    VariableDecl {
        name: String,
        r#type: String,
        initializer: Option<Box<Expression>>,
        line: usize,
    },
}

#[derive(Debug, Clone)]
pub enum Expression {
    BinaryOp {
        operator: String,
        left: Box<Expression>,
        right: Box<Expression>,
        line: usize,
    },
    UnaryOp {
        operator: String,
        operand: Box<Expression>,
        line: usize,
    },
    TernaryOp {
        condition: Box<Expression>,
        true_expr: Box<Expression>,
        false_expr: Box<Expression>,
        line: usize,
    },
    FunctionCall {
        name: String,
        args: Vec<Expression>,
        line: usize,
    },
    MethodCall {
        object: Box<Expression>,
        method: String,
        args: Vec<Expression>,
        line: usize,
    },
    ArrayAccess {
        base: Box<Expression>,
        index: Box<Expression>,
        line: usize,
    },
    MemberAccess {
        object: Box<Expression>,
        field: String,
        line: usize,
    },
    PointerMemberAccess {
        object: Box<Expression>,
        field: String,
        line: usize,
    },
    VariableRef {
        name: String,
        line: usize,
    },
    NumberLiteral {
        value: String,
        line: usize,
    },
    StringLiteral {
        value: String,
        line: usize,
    },
    CharLiteral {
        value: String,
        line: usize,
    },
    BoolLiteral {
        value: bool,
        line: usize,
    },
    Parenthesized {
        expr: Box<Expression>,
        line: usize,
    },
}

impl Expression {
    pub fn line(&self) -> usize {
        match self {
            Expression::BinaryOp { line, .. }
            | Expression::UnaryOp { line, .. }
            | Expression::TernaryOp { line, .. }
            | Expression::FunctionCall { line, .. }
            | Expression::MethodCall { line, .. }
            | Expression::ArrayAccess { line, .. }
            | Expression::MemberAccess { line, .. }
            | Expression::PointerMemberAccess { line, .. }
            | Expression::VariableRef { line, .. }
            | Expression::NumberLiteral { line, .. }
            | Expression::StringLiteral { line, .. }
            | Expression::CharLiteral { line, .. }
            | Expression::BoolLiteral { line, .. }
            | Expression::Parenthesized { line, .. } => *line,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ForbiddenKeyword {
    pub keyword: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct ConstraintViolation {
    pub violation_type: String, // "traditional_for_loop", "raw_pointer_usage", etc.
    pub line: usize,
    pub start_char: usize,
    pub end_char: usize,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
    pub error_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForLoopType {
    Range,
    Traditional,
}
