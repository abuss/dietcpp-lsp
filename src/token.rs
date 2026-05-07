/// Token types for the DietC++ lexer
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Keyword { value: String, line: usize, column: usize },
    Identifier { value: String, line: usize, column: usize },
    Number { value: String, line: usize, column: usize },
    String { value: String, line: usize, column: usize },
    Char { value: String, line: usize, column: usize },
    Operator { value: String, line: usize, column: usize },
    Preprocessor { text: String, line: usize, column: usize },
    EOF,
}

impl Token {
    pub fn line(&self) -> usize {
        match self {
            Token::Keyword { line, .. }
            | Token::Identifier { line, .. }
            | Token::Number { line, .. }
            | Token::String { line, .. }
            | Token::Char { line, .. }
            | Token::Operator { line, .. }
            | Token::Preprocessor { line, .. } => *line,
            Token::EOF => 0,
        }
    }

    pub fn column(&self) -> usize {
        match self {
            Token::Keyword { column, .. }
            | Token::Identifier { column, .. }
            | Token::Number { column, .. }
            | Token::String { column, .. }
            | Token::Char { column, .. }
            | Token::Operator { column, .. }
            | Token::Preprocessor { column, .. } => *column,
            Token::EOF => 0,
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Keyword { value, .. } => write!(f, "Keyword({})", value),
            Token::Identifier { value, .. } => write!(f, "Identifier({})", value),
            Token::Number { value, .. } => write!(f, "Number({})", value),
            Token::String { value, .. } => write!(f, "String({})", value),
            Token::Char { value, .. } => write!(f, "Char({})", value),
            Token::Operator { value, .. } => write!(f, "Operator({})", value),
            Token::Preprocessor { text, .. } => write!(f, "Preprocessor({})", text),
            Token::EOF => write!(f, "EOF"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokenizerError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl fmt::Display for TokenizerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}

pub struct Tokenizer {
    source: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

const KEYWORDS: &[&str] = &[
    "class", "struct", "enum", "namespace", "if", "else", "while", "for", "do", "auto", "int",
    "float", "double", "char", "bool", "void", "const", "return", "break", "continue", "public",
    "private", "protected", "using", "switch", "case", "default", "true", "false", "friend",
    "register", "throw", "try", "catch", "final", "virtual", "goto", "mutable", "extern",
    "inline", "static", "typedef",
];

impl Tokenizer {
    pub fn new(source: &str) -> Self {
        Tokenizer {
            source: source.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, TokenizerError> {
        let mut tokens = Vec::new();

        while self.position < self.source.len() {
            self.skip_whitespace_and_comments()?;

            if self.position >= self.source.len() {
                break;
            }

            let ch = self.current_char();

            if ch == '#' {
                tokens.push(self.read_preprocessor()?);
            } else if ch == '"' {
                tokens.push(self.read_string()?);
            } else if ch == '\'' {
                tokens.push(self.read_char()?);
            } else if is_digit(ch) {
                tokens.push(self.read_number()?);
            } else if is_identifier_start(ch) {
                tokens.push(self.read_identifier_or_keyword()?);
            } else if is_operator_char(ch) {
                tokens.push(self.read_operator()?);
            } else {
                return Err(TokenizerError {
                    line: self.line,
                    column: self.column,
                    message: format!("Unexpected character: '{}'", ch),
                });
            }
        }

        tokens.push(Token::EOF);
        Ok(tokens)
    }

    fn current_char(&self) -> char {
        if self.position < self.source.len() {
            self.source[self.position]
        } else {
            '\0'
        }
    }

    fn peek_char(&self, offset: usize) -> char {
        if self.position + offset < self.source.len() {
            self.source[self.position + offset]
        } else {
            '\0'
        }
    }

    fn advance(&mut self) -> char {
        let ch = self.current_char();
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        self.position += 1;
        ch
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<(), TokenizerError> {
        loop {
            match self.current_char() {
                ' ' | '\t' | '\n' | '\r' => {
                    self.advance();
                }
                '/' if self.peek_char(1) == '/' => {
                    // Line comment
                    while self.current_char() != '\n' && self.current_char() != '\0' {
                        self.advance();
                    }
                    if self.current_char() == '\n' {
                        self.advance();
                    }
                }
                '/' if self.peek_char(1) == '*' => {
                    // Block comment
                    self.advance(); // consume '/'
                    self.advance(); // consume '*'
                    while self.position < self.source.len() {
                        if self.current_char() == '*' && self.peek_char(1) == '/' {
                            self.advance(); // consume '*'
                            self.advance(); // consume '/'
                            break;
                        }
                        self.advance();
                    }
                    if self.position >= self.source.len() {
                        return Err(TokenizerError {
                            line: self.line,
                            column: self.column,
                            message: "Unclosed block comment".to_string(),
                        });
                    }
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn read_preprocessor(&mut self) -> Result<Token, TokenizerError> {
        let line = self.line;
        let column = self.column;
        let mut text = String::new();

        while self.current_char() != '\n' && self.current_char() != '\0' {
            text.push(self.advance());
        }
        if self.current_char() == '\n' {
            self.advance();
        }

        Ok(Token::Preprocessor { text, line, column })
    }

    fn read_string(&mut self) -> Result<Token, TokenizerError> {
        let line = self.line;
        let column = self.column;
        let mut value = String::new();

        self.advance(); // consume opening quote

        while self.current_char() != '"' && self.current_char() != '\0' {
            if self.current_char() == '\\' {
                self.advance();
                match self.current_char() {
                    'n' => {
                        value.push('\n');
                        self.advance();
                    }
                    't' => {
                        value.push('\t');
                        self.advance();
                    }
                    '\\' => {
                        value.push('\\');
                        self.advance();
                    }
                    '"' => {
                        value.push('"');
                        self.advance();
                    }
                    '\'' => {
                        value.push('\'');
                        self.advance();
                    }
                    _ => {
                        value.push(self.advance());
                    }
                }
            } else {
                value.push(self.advance());
            }
        }

        if self.current_char() != '"' {
            return Err(TokenizerError {
                line,
                column,
                message: "Unclosed string literal".to_string(),
            });
        }
        self.advance(); // consume closing quote

        Ok(Token::String { value, line, column })
    }

    fn read_char(&mut self) -> Result<Token, TokenizerError> {
        let line = self.line;
        let column = self.column;
        let mut value = String::new();

        self.advance(); // consume opening quote

        while self.current_char() != '\'' && self.current_char() != '\0' {
            if self.current_char() == '\\' {
                self.advance();
                match self.current_char() {
                    'n' => {
                        value.push('\n');
                        self.advance();
                    }
                    't' => {
                        value.push('\t');
                        self.advance();
                    }
                    '\\' => {
                        value.push('\\');
                        self.advance();
                    }
                    '"' => {
                        value.push('"');
                        self.advance();
                    }
                    '\'' => {
                        value.push('\'');
                        self.advance();
                    }
                    _ => {
                        value.push(self.advance());
                    }
                }
            } else {
                value.push(self.advance());
            }
        }

        if self.current_char() != '\'' {
            return Err(TokenizerError {
                line,
                column,
                message: "Unclosed char literal".to_string(),
            });
        }
        self.advance(); // consume closing quote

        Ok(Token::Char { value, line, column })
    }

    fn read_number(&mut self) -> Result<Token, TokenizerError> {
        let line = self.line;
        let column = self.column;
        let mut value = String::new();

        // Read integer part
        while is_digit(self.current_char()) {
            value.push(self.advance());
        }

        // Check for hex
        if value == "0" && (self.current_char() == 'x' || self.current_char() == 'X') {
            value.push(self.advance()); // consume 'x'
            while is_hex_digit(self.current_char()) {
                value.push(self.advance());
            }
        } else {
            // Check for decimal point
            if self.current_char() == '.' && is_digit(self.peek_char(1)) {
                value.push(self.advance()); // consume '.'
                while is_digit(self.current_char()) {
                    value.push(self.advance());
                }
            }

            // Check for scientific notation
            if self.current_char() == 'e' || self.current_char() == 'E' {
                value.push(self.advance()); // consume 'e'
                if self.current_char() == '+' || self.current_char() == '-' {
                    value.push(self.advance());
                }
                while is_digit(self.current_char()) {
                    value.push(self.advance());
                }
            }
        }

        Ok(Token::Number { value, line, column })
    }

    fn read_identifier_or_keyword(&mut self) -> Result<Token, TokenizerError> {
        let line = self.line;
        let column = self.column;
        let mut value = String::new();

        while is_identifier_char(self.current_char()) {
            value.push(self.advance());
        }

        let token = if KEYWORDS.contains(&value.as_str()) {
            Token::Keyword { value, line, column }
        } else {
            Token::Identifier { value, line, column }
        };

        Ok(token)
    }

    fn read_operator(&mut self) -> Result<Token, TokenizerError> {
        let line = self.line;
        let column = self.column;

        // Try two-character operators first
        let two_char = format!("{}{}", self.current_char(), self.peek_char(1));
        let value = if matches!(
            two_char.as_str(),
            "<<" | ">>" | "<=" | ">=" | "==" | "!=" | "->" | "::" | "||" | "&&" | "++" | "--"
        ) {
            self.advance();
            self.advance();
            two_char
        } else {
            self.advance().to_string()
        };

        Ok(Token::Operator { value, line, column })
    }
}

fn is_digit(ch: char) -> bool {
    ch >= '0' && ch <= '9'
}

fn is_hex_digit(ch: char) -> bool {
    is_digit(ch) || (ch >= 'a' && ch <= 'f') || (ch >= 'A' && ch <= 'F')
}

fn is_identifier_start(ch: char) -> bool {
    (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || ch == '_'
}

fn is_identifier_char(ch: char) -> bool {
    is_identifier_start(ch) || is_digit(ch)
}

fn is_operator_char(ch: char) -> bool {
    matches!(
        ch,
        '+' | '-' | '*' | '/' | '%' | '=' | '<' | '>' | '!' | '&' | '|' | '^' | '~' | '('
            | ')' | '{' | '}' | '[' | ']' | ';' | ':' | '.' | ',' | '?'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple_function() {
        let source = "int main() { return 0; }";
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer.tokenize().unwrap();

        assert!(matches!(&tokens[0], Token::Keyword { value, .. } if value == "int"));
        assert!(matches!(&tokens[1], Token::Identifier { value, .. } if value == "main"));
        assert!(matches!(&tokens[2], Token::Operator { value, .. } if value == "("));
        assert!(matches!(&tokens[3], Token::Operator { value, .. } if value == ")"));
        assert!(matches!(&tokens[4], Token::Operator { value, .. } if value == "{"));
        assert!(matches!(&tokens[5], Token::Keyword { value, .. } if value == "return"));
        assert!(matches!(&tokens[6], Token::Number { value, .. } if value == "0"));
        assert!(matches!(&tokens[7], Token::Operator { value, .. } if value == ";"));
        assert!(matches!(&tokens[8], Token::Operator { value, .. } if value == "}"));
        assert!(matches!(&tokens[9], Token::EOF));
    }

    #[test]
    fn test_tokenize_keywords() {
        let source = "virtual void static";
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer.tokenize().unwrap();

        let keywords: Vec<String> = tokens
            .iter()
            .filter_map(|t| match t {
                Token::Keyword { value, .. } => Some(value.clone()),
                _ => None,
            })
            .collect();

        assert_eq!(keywords.len(), 3);
        assert_eq!(keywords[0], "virtual");
        assert_eq!(keywords[1], "void");
        assert_eq!(keywords[2], "static");
    }

    #[test]
    fn test_tokenize_operators() {
        let source = "a++ && b-- || c->d";
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer.tokenize().unwrap();

        let operators: Vec<String> = tokens
            .iter()
            .filter_map(|t| match t {
                Token::Operator { value, .. } => Some(value.clone()),
                _ => None,
            })
            .collect();

        assert!(operators.contains(&"++".to_string()));
        assert!(operators.contains(&"&&".to_string()));
        assert!(operators.contains(&"||".to_string()));
        assert!(operators.contains(&"->".to_string()));
    }

    #[test]
    fn test_tokenize_numbers() {
        let source = "int x = 42; float y = 3.14; int z = 0xFF; double w = 1e-3;";
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer.tokenize().unwrap();

        let numbers: Vec<String> = tokens
            .iter()
            .filter_map(|t| match t {
                Token::Number { value, .. } => Some(value.clone()),
                _ => None,
            })
            .collect();

        assert!(numbers.contains(&"42".to_string()));
        assert!(numbers.contains(&"3.14".to_string()));
        assert!(numbers.contains(&"0xFF".to_string()));
        assert!(numbers.contains(&"1e-3".to_string()));
    }

    #[test]
    fn test_tokenize_strings() {
        let source = r#"const char* str = "hello";"#;
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer.tokenize().unwrap();

        let string_token = tokens
            .iter()
            .find(|t| matches!(t, Token::String { .. }))
            .unwrap();
        assert!(matches!(string_token, Token::String { value, .. } if value == "hello"));
    }

    #[test]
    fn test_tokenize_preprocessor() {
        let source = "#include <iostream>\nint main() {}";
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer.tokenize().unwrap();

        assert!(matches!(&tokens[0], Token::Preprocessor { .. }));
    }

    #[test]
    fn test_tokenize_comments() {
        let source = "int x = 5; // line comment\n/* block */ int y;";
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer.tokenize().unwrap();

        // Should skip comments and tokenize the rest
        assert!(tokens.iter().any(|t| matches!(t, Token::Identifier { value, .. } if value == "x")));
        assert!(tokens.iter().any(|t| matches!(t, Token::Identifier { value, .. } if value == "y")));
    }

    #[test]
    fn test_tokenize_for_loop_range_based() {
        let source = "for (auto& x : container) {}";
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer.tokenize().unwrap();

        // Should tokenize successfully
        assert!(tokens.iter().any(|t| matches!(t, Token::Operator { value, .. } if value == ":")));
    }

    #[test]
    fn test_tokenize_for_loop_traditional() {
        let source = "for (int i = 0; i < 10; i++) {}";
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer.tokenize().unwrap();

        // Should tokenize all parts
        assert!(tokens.iter().any(|t| matches!(t, Token::Keyword { value, .. } if value == "int")));
        assert!(tokens.iter().any(|t| matches!(t, Token::Operator { value, .. } if value == "=")));
        assert!(tokens.iter().any(|t| matches!(t, Token::Operator { value, .. } if value == ";")));
    }

    #[test]
    fn test_scope_resolution() {
        let source = "std::vector<int> v;";
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer.tokenize().unwrap();

        assert!(tokens.iter().any(|t| matches!(t, Token::Operator { value, .. } if value == "::")));
    }

    #[test]
    fn test_bitwise_operators() {
        let source = "int a = x | y ^ z & w ~ b;";
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer.tokenize().unwrap();

        let bitwise_ops: Vec<_> = tokens
            .iter()
            .filter_map(|t| match t {
                Token::Operator { value, .. } => Some(value.as_str()),
                _ => None,
            })
            .collect();

        assert!(bitwise_ops.contains(&"|"));
        assert!(bitwise_ops.contains(&"^"));
        assert!(bitwise_ops.contains(&"&"));
        assert!(bitwise_ops.contains(&"~"));
    }
}
