//! QuickBB CLI.
//!
//! Reads an undirected graph as an edge list from stdin (or a file given as the
//! first argument) and prints the treewidth plus a witness elimination ordering.
//!
//! # Input format
//!
//! Each line contains two whitespace-separated integers `u v` denoting an
//! undirected edge `{u, v}`. Blank lines and lines beginning with `#` are
//! ignored. Indexing is auto-detected: if every vertex label is `>= 1`, the
//! labels are interpreted as 1-indexed and shifted down by one; otherwise
//! 0-indexing is assumed.
//!
//! # Example
//!
//! ```text
//! $ printf '0 1\n1 2\n2 3\n3 4\n' | quickbb
//! treewidth = 1
//! optimal   = true
//! order     = 0 1 2 3 4
//! ```

use std::io::{self, Read};
use std::process::ExitCode;

use quickbb::graph::Graph;
use quickbb::treewidth;

fn main() -> ExitCode {
    let raw = match read_input() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading input: {e}");
            return ExitCode::FAILURE;
        }
    };

    let edges = match parse_edges(&raw) {
        Ok(es) => es,
        Err(e) => {
            eprintln!("error parsing edges: {e}");
            return ExitCode::FAILURE;
        }
    };

    if edges.is_empty() {
        eprintln!("error: no edges in input");
        return ExitCode::FAILURE;
    }

    let g = Graph::from_edges(edges.iter().copied());
    let result = treewidth(&g);

    println!("treewidth = {}", result.treewidth);
    println!("optimal   = {}", result.optimal);
    print!("order     =");
    for v in &result.order {
        print!(" {}", v.index());
    }
    println!();
    ExitCode::SUCCESS
}

/// Read all of stdin, or the file named by `argv[1]` if present.
fn read_input() -> io::Result<String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 {
        std::fs::read_to_string(&args[1])
    } else {
        let mut s = String::new();
        io::stdin().read_to_string(&mut s)?;
        Ok(s)
    }
}

/// Parse the edge list. Returns `(edges, offset)` semantics hidden inside:
/// the returned edges use 0-indexed vertices.
fn parse_edges(raw: &str) -> Result<Vec<(usize, usize)>, String> {
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for (lineno, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let a = parts
            .next()
            .ok_or_else(|| format!("line {lineno}: missing first endpoint"))?;
        let b = parts
            .next()
            .ok_or_else(|| format!("line {lineno}: missing second endpoint"))?;
        let rest = parts.next();
        if let Some(r) = rest {
            return Err(format!("line {lineno}: unexpected extra token '{r}'"));
        }
        let u: usize = a
            .parse()
            .map_err(|_| format!("line {lineno}: not an integer: '{a}'"))?;
        let v: usize = b
            .parse()
            .map_err(|_| format!("line {lineno}: not an integer: '{b}'"))?;
        pairs.push((u, v));
    }

    // Auto-detect 1-indexing: if no label is 0, shift everything down.
    let any_zero = pairs.iter().any(|&(u, v)| u == 0 || v == 0);
    if !any_zero && !pairs.is_empty() {
        for (u, v) in pairs.iter_mut() {
            *u = u.saturating_sub(1);
            *v = v.saturating_sub(1);
        }
    }

    Ok(pairs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_zero_indexed() {
        let es = parse_edges("0 1\n1 2\n").unwrap();
        assert_eq!(es, vec![(0, 1), (1, 2)]);
    }

    #[test]
    fn parse_one_indexed_shifted() {
        let es = parse_edges("1 2\n2 3\n").unwrap();
        assert_eq!(es, vec![(0, 1), (1, 2)]);
    }

    #[test]
    fn parse_ignores_comments_and_blanks() {
        let es = parse_edges("# header\n\n0 1  \n  2 3\n").unwrap();
        assert_eq!(es, vec![(0, 1), (2, 3)]);
    }

    #[test]
    fn parse_rejects_extra_token() {
        assert!(parse_edges("0 1 2\n").is_err());
    }
}
