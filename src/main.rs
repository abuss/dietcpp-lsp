use dietcpp_lsp::lsp::LspServer;
use std::io;
use std::fs::OpenOptions;
use std::io::Write;

fn main() -> io::Result<()> {
    // Log to file for debugging
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/tmp/dietcpp_lsp_startup.log") {
        let _ = writeln!(file, "[DietC++ LSP] Server starting...");
    }
    
    eprintln!("[DietC++ LSP] Server starting...");
    
    // Initialize and run the LSP server
    let mut server = LspServer::new();
    eprintln!("[DietC++ LSP] Server initialized");
    server.run()?;
    Ok(())
}
