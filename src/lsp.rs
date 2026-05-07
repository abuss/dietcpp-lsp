/// LSP (Language Server Protocol) implementation for DietC++
use serde_json::json;
use std::io::{self, BufRead, BufReader, Write};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::path::PathBuf;
use lsp_types::*;
use crate::parser::Parser;
use crate::token::Tokenizer;
use crate::code_actions::{generate_code_actions, generate_semantic_tokens};
use crate::config::DietCppConfig;
use url::Url;

pub struct LspServer {
    #[allow(dead_code)]
    request_id: i64,
    documents: Arc<Mutex<HashMap<String, String>>>,
    initialized: bool,
    workspace_root: Option<PathBuf>,
    config: DietCppConfig,
}

impl LspServer {
    pub fn new() -> Self {
        LspServer {
            request_id: 0,
            documents: Arc::new(Mutex::new(HashMap::new())),
            initialized: false,
            workspace_root: None,
            config: DietCppConfig::default(),
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin.lock());
        let mut line_buffer = String::new();

        loop {
            line_buffer.clear();
            
            // Read lines until we find Content-Length header
            loop {
                match reader.read_line(&mut line_buffer)? {
                    0 => {
                        // EOF reached
                        return Ok(());
                    }
                    _ => {
                        if line_buffer.starts_with("Content-Length:") {
                            break;
                        }
                        if !line_buffer.trim().is_empty() {
                        }
                        line_buffer.clear();
                    }
                }
            }

            // Parse Content-Length
            let content_length: usize = line_buffer
                .strip_prefix("Content-Length:")
                .unwrap()
                .trim()
                .parse()
                .unwrap_or(0);


            // Read blank line
            line_buffer.clear();
            reader.read_line(&mut line_buffer)?;
            if !line_buffer.trim().is_empty() {
            }

            // Read message body
            let mut buffer = vec![0; content_length];
            match io::Read::read_exact(&mut reader, &mut buffer) {
                Ok(_) => {
                    let message = String::from_utf8_lossy(&buffer);
                    self.handle_message(&message)?;
                }
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                    return Ok(());
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    fn handle_message(&mut self, message: &str) -> io::Result<()> {
        match serde_json::from_str::<serde_json::Value>(message) {
            Ok(json) => {
                if let Some(method) = json.get("method").and_then(|v| v.as_str()) {
                    match method {
                        "initialize" => self.handle_initialize(&json)?,
                        "initialized" => self.handle_initialized(&json)?,
                        "textDocument/didOpen" => self.handle_did_open(&json)?,
                        "textDocument/didChange" => self.handle_did_change(&json)?,
                        "textDocument/didClose" => self.handle_did_close(&json)?,
                        "textDocument/codeAction" => self.handle_code_action(&json)?,
                        "textDocument/semanticTokens/full" => self.handle_semantic_tokens(&json)?,
                        "shutdown" => self.handle_shutdown(&json)?,
                        "exit" => {
                            self.send_response(json.get("id"), json!(null));
                            std::process::exit(0);
                        }
                        _ => {
                            // Unknown method - send error response
                            if let Some(id) = json.get("id") {
                                self.send_error_response(
                                    id,
                                    -32601,
                                    "Method not found".to_string(),
                                );
                            }
                        }
                    }
                }
            }
            Err(_) => {
            }
        }

        Ok(())
    }

     fn handle_initialize(&mut self, json: &serde_json::Value) -> io::Result<()> {
         self.initialized = true;

         // Extract rootPath or rootUri from initialization params
          if let Some(params) = json.get("params") {
              // Try rootUri first (preferred), then rootPath (legacy)
              if let Some(root_uri) = params.get("rootUri").and_then(|v| v.as_str()) {
                  // Convert file:// URI to path
                  if let Ok(path) = Url::parse(root_uri) {
                      if let Ok(path_buf) = path.to_file_path() {
                          self.workspace_root = Some(path_buf.clone());
                           // Try to load config from workspace root
                           match DietCppConfig::load(&path_buf) {
                               Ok(config) => {
                                   self.config = config;
                               }
                               Err(_) => {
                                   self.config = DietCppConfig::default();
                               }
                           }
                      }
                  }
              } else if let Some(root_path) = params.get("rootPath").and_then(|v| v.as_str()) {
                  // Legacy rootPath
                  let path_buf = PathBuf::from(root_path);
                  self.workspace_root = Some(path_buf.clone());
                   // Try to load config from workspace root
                   match DietCppConfig::load(&path_buf) {
                       Ok(config) => {
                           self.config = config;
                       }
                       Err(_) => {
                           self.config = DietCppConfig::default();
                       }
                   }
              }
             
             // Apply initializationOptions from client (VS Code settings override file config)
             if let Some(init_options) = params.get("initializationOptions") {
                 // Apply rules settings if provided
                 if let Some(rules) = init_options.get("rules") {
                     if let Some(val) = rules.get("preprocessor_directives").and_then(|v| v.as_bool()) {
                         self.config.rules.preprocessor_directives = val;
                     }
                     if let Some(val) = rules.get("forbidden_keywords").and_then(|v| v.as_bool()) {
                         self.config.rules.forbidden_keywords = val;
                     }
                     if let Some(val) = rules.get("traditional_for_loops").and_then(|v| v.as_bool()) {
                         self.config.rules.traditional_for_loops = val;
                     }
                     if let Some(val) = rules.get("raw_pointers").and_then(|v| v.as_bool()) {
                         self.config.rules.raw_pointers = val;
                     }
                 }
                 
                 // Apply general settings if provided
                 if let Some(general) = init_options.get("general") {
                     if let Some(severity) = general.get("severity").and_then(|v| v.as_str()) {
                         self.config.general.severity = severity.to_string();
                     }
                 }
             }
         }

         let capabilities = json!({
             "capabilities": {
                 "textDocumentSync": 1,  // FULL sync
                 "codeActionProvider": true,
                 "semanticTokensProvider": {
                     "legend": {
                         "tokenTypes": [
                             "namespace", "type", "class", "enum", "interface",
                             "struct", "typeParameter", "parameter", "variable",
                             "property", "enumMember", "event", "function",
                             "method", "macro", "keyword", "comment",
                             "string", "number", "regexp", "operator"
                         ],
                         "tokenModifiers": ["declaration", "definition", "readonly", "static"]
                     },
                     "full": true
                 }
             },
              "serverInfo": {
                   "name": "DietC++ LSP",
                   "version": "0.2.0"
               }
         });

         self.send_response(json.get("id"), capabilities);
         Ok(())
     }

    fn handle_initialized(&mut self, _json: &serde_json::Value) -> io::Result<()> {
        // Server is initialized, ready to send diagnostics
        Ok(())
    }

    fn handle_did_open(&mut self, json: &serde_json::Value) -> io::Result<()> {
        if let Some(params) = json.get("params") {
            if let Some(text_document) = params.get("textDocument") {
                if let Some(uri) = text_document.get("uri").and_then(|v| v.as_str()) {
                    if let Some(text) = text_document.get("text").and_then(|v| v.as_str()) {
                        // Store document
                        {
                            let mut docs = self.documents.lock().unwrap();
                            docs.insert(uri.to_string(), text.to_string());
                        }

                        // Parse and send diagnostics
                        self.publish_diagnostics(uri, text)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_did_change(&mut self, json: &serde_json::Value) -> io::Result<()> {
        if let Some(params) = json.get("params") {
            if let Some(text_document) = params.get("textDocument") {
                if let Some(uri) = text_document.get("uri").and_then(|v| v.as_str()) {
                    if let Some(content_changes) = params.get("contentChanges").and_then(|v| v.as_array()) {
                        if let Some(change) = content_changes.first() {
                            if let Some(text) = change.get("text").and_then(|v| v.as_str()) {
                                // Update document
                                {
                                    let mut docs = self.documents.lock().unwrap();
                                    docs.insert(uri.to_string(), text.to_string());
                                }

                                // Parse and send diagnostics
                                self.publish_diagnostics(uri, text)?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_did_close(&mut self, json: &serde_json::Value) -> io::Result<()> {
        if let Some(params) = json.get("params") {
            if let Some(text_document) = params.get("textDocument") {
                if let Some(uri) = text_document.get("uri").and_then(|v| v.as_str()) {
                    let mut docs = self.documents.lock().unwrap();
                    docs.remove(uri);

                    // Clear diagnostics
                    self.clear_diagnostics(uri)?;
                }
            }
        }

        Ok(())
    }

    fn handle_shutdown(&mut self, json: &serde_json::Value) -> io::Result<()> {
        self.send_response(json.get("id"), json!(null));
        Ok(())
    }

    fn handle_code_action(&self, json: &serde_json::Value) -> io::Result<()> {
        if let Some(params) = json.get("params") {
            if let Some(text_document) = params.get("textDocument") {
                if let Some(uri) = text_document.get("uri").and_then(|v| v.as_str()) {
                    // Get the document
                    let docs = self.documents.lock().unwrap();
                    if let Some(source) = docs.get(uri) {
                        // Parse and get violations
                        let mut tokenizer = Tokenizer::new(source);
                        if let Ok(tokens) = tokenizer.tokenize() {
                            let mut parser = Parser::new(tokens);
                            let ast = parser.parse();

                            // Generate code actions
                            let actions = generate_code_actions(&ast.constraint_violations, uri);
                            self.send_response(json.get("id"), json!(actions));
                            return Ok(());
                        }
                    }
                }
            }
        }

        self.send_response(json.get("id"), json!([]));
        Ok(())
    }

    fn handle_semantic_tokens(&self, json: &serde_json::Value) -> io::Result<()> {
        if let Some(params) = json.get("params") {
            if let Some(text_document) = params.get("textDocument") {
                if let Some(uri) = text_document.get("uri").and_then(|v| v.as_str()) {
                    // Get the document
                    let docs = self.documents.lock().unwrap();
                    if let Some(source) = docs.get(uri) {
                        // Parse and generate semantic tokens
                        let mut tokenizer = Tokenizer::new(source);
                        if let Ok(tokens) = tokenizer.tokenize() {
                            let mut parser = Parser::new(tokens);
                            let ast = parser.parse();

                            let semantic_tokens = generate_semantic_tokens(&ast);
                            self.send_response(json.get("id"), json!(semantic_tokens));
                            return Ok(());
                        }
                    }
                }
            }
        }

        self.send_response(json.get("id"), json!(null));
        Ok(())
    }

    fn publish_diagnostics(&self, uri: &str, source: &str) -> io::Result<()> {
        // Debug logging to file
        use std::fs::OpenOptions;
        use std::io::Write;
        let mut debug_file = OpenOptions::new().create(true).append(true).open("/tmp/lsp_diagnostics.log").ok();
        if let Some(ref mut f) = debug_file {
            let _ = writeln!(f, "[LSP] publish_diagnostics called for: {}", uri);
            let _ = writeln!(f, "[LSP] Source length: {} bytes", source.len());
        }
        
         
          // Tokenize then parse the source code
          let mut tokenizer = Tokenizer::new(source);
          match tokenizer.tokenize() {
              Ok(tokens) => {
                 
                 let mut parser = Parser::with_config(tokens, self.config.clone());
                 let ast = parser.parse();
                
                // Write parser results to separate log file for debugging
                use std::fs::OpenOptions;
                use std::io::Write;
                if let Ok(mut parser_log) = OpenOptions::new().create(true).append(true).open("/tmp/parser_results.log") {
                    let _ = writeln!(parser_log, "=== Parser Results ===");
                    let _ = writeln!(parser_log, "Items: {}", ast.items.len());
                    let _ = writeln!(parser_log, "Violations: {}", ast.constraint_violations.len());
                    for v in &ast.constraint_violations {
                        let _ = writeln!(parser_log, "  - Line {}: {}", v.line, v.violation_type);
                    }
                 }
                 
                 if let Some(ref mut f) = debug_file {
                     let _ = writeln!(f, "[LSP] Parser returned");
                     let _ = writeln!(f, "[LSP] Found {} violations", ast.constraint_violations.len());
                     for v in &ast.constraint_violations {
                         let _ = writeln!(f, "[LSP]   - Line {}: {} (chars {}-{}) - {}", v.line, v.violation_type, v.start_char, v.end_char, v.message);
                     }
                 }

                 // Convert violations to LSP diagnostics
                let diagnostics: Vec<Diagnostic> = ast
                    .constraint_violations
                    .iter()
                    .map(|violation| {
                        Diagnostic {
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
                            code: Some(NumberOrString::String(violation.violation_type.clone())),
                            source: Some("DietC++".to_string()),
                            message: violation.message.clone(),
                            related_information: None,
                            tags: None,
                            code_description: None,
                            data: None,
                        }
                    })
                    .collect();


                // Send diagnostics notification
                let notification = json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/publishDiagnostics",
                    "params": {
                        "uri": uri,
                        "diagnostics": diagnostics
                    }
                });

                self.send_notification(notification)?;
            }
            Err(e) => {
                // Send error diagnostic
                let notification = json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/publishDiagnostics",
                    "params": {
                        "uri": uri,
                        "diagnostics": [
                            {
                                "range": {
                                    "start": { "line": 0, "character": 0 },
                                    "end": { "line": 0, "character": 1 }
                                },
                                "severity": 1,
                                "code": "parse_error",
                                "source": "DietC++",
                                "message": format!("Parse error: {}", e)
                            }
                        ]
                    }
                });

                self.send_notification(notification)?;
            }
        }

        Ok(())
    }

    fn clear_diagnostics(&self, uri: &str) -> io::Result<()> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": uri,
                "diagnostics": []
            }
        });

        self.send_notification(notification)?;
        Ok(())
    }

    fn send_response(&self, id: Option<&serde_json::Value>, result: serde_json::Value) {
        let response = json!({
            "jsonrpc": "2.0",
            "id": id.cloned().unwrap_or(json!(null)),
            "result": result
        });

        self.send_message(&response).ok();
    }

    fn send_error_response(&self, id: &serde_json::Value, code: i32, message: String) {
        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code,
                "message": message
            }
        });

        self.send_message(&response).ok();
    }

    fn send_notification(&self, notification: serde_json::Value) -> io::Result<()> {
        self.send_message(&notification)
    }

    fn send_message(&self, message: &serde_json::Value) -> io::Result<()> {
        let content = serde_json::to_string(message).unwrap();
        let content_length = content.len();

        let mut stdout = io::stdout();
        write!(
            stdout,
            "Content-Length: {}\r\n\r\n{}",
            content_length, content
        )?;
        stdout.flush()?;

        Ok(())
    }
}

impl Default for LspServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_server_creation() {
        let server = LspServer::new();
        assert!(!server.initialized);
        assert_eq!(server.request_id, 0);
    }

    #[test]
    fn test_lsp_server_initialization() {
        let mut server = LspServer::new();
        assert!(!server.initialized);
    }

    #[test]
    fn test_document_storage() {
        let server = LspServer::new();
        {
            let mut docs = server.documents.lock().unwrap();
            docs.insert("file:///test.cpp".to_string(), "int x = 5;".to_string());
        }

        {
            let docs = server.documents.lock().unwrap();
            assert!(docs.contains_key("file:///test.cpp"));
        }
    }
}
