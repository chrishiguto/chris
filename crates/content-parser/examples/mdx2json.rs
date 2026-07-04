//! Prints a parsed `.mdx` document as AST JSON — the same payload the
//! pipeline will store under `post:{slug}`; used to hand-seed KV in Slice 3.
//!
//! Usage: `cargo run -p content-parser --example mdx2json -- <path/to/index.mdx>`

use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: mdx2json <path/to/index.mdx>");
        return ExitCode::FAILURE;
    };
    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("{path}: {err}");
            return ExitCode::FAILURE;
        }
    };
    match content_parser::parse_named(&source, &path) {
        Ok(doc) => match serde_json::to_string_pretty(&doc) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("{path}: failed to serialize: {err}");
                ExitCode::FAILURE
            }
        },
        Err(diags) => {
            for diag in diags {
                eprintln!("{diag}");
            }
            ExitCode::FAILURE
        }
    }
}
