//! snitch-mcp: the local stdio MCP server.
//!
//! There is no `snitch mcp` subcommand. `snitch/cli.rs` explains why: the name is the interface,
//! and a protocol server is the last thing that should be buried inside another command.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--version") => {
            println!("snitch-mcp {}", snitch::VERSION);
            ExitCode::SUCCESS
        }
        Some("--help") | Some("-h") => {
            println!(
                "usage: snitch-mcp\n\n\
                 A local Model Context Protocol server over stdio. It takes no arguments: an MCP\n\
                 host launches it and speaks JSON-RPC 2.0 over the pipe.\n\n\
                 Everything runs on this machine. No network call is made.\n\n\
                 Configure it by pointing your host at this binary's absolute path."
            );
            ExitCode::SUCCESS
        }
        Some(other) => {
            snitch::mcp::log(&format!("unexpected argument {other:?}; run with --help"));
            ExitCode::from(2)
        }
        None => match snitch::mcp::serve() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                snitch::mcp::log(&format!("stdio closed: {e}"));
                ExitCode::FAILURE
            }
        },
    }
}
