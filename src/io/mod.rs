//! File I/O for graphs: Graph3 import/export plus thin file helpers.

pub mod graph3;

use crate::Graph;
use std::fs::File;
use std::io::{Error, Write};

/// Write `g` to `filename` in Graph3 format (see [`graph3`]).
pub fn export_graph3(g: &Graph, filename: &str) -> Result<(), Error> {
    let mut file = File::create(filename)?;
    write!(file, "{}", g.to_graph3())?;
    Ok(())
}

/// Read a graph from `filename` in Graph3 format (see [`graph3`]).
///
/// Parse errors carry the offending 1-based line number.
pub fn import_graph3(filename: &str) -> Result<Graph, String> {
    graph3::from_graph3_file(filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "cartographer_io_{name}_{}.graph3",
            std::process::id()
        ))
    }

    #[test]
    fn export_import_round_trip() {
        let g = graph3::from_graph3("1 2 : H\n3 : Z(4)\n1 3 2\n").unwrap();
        let path = tmp("rt");
        export_graph3(&g, path.to_str().unwrap()).unwrap();
        let back = import_graph3(path.to_str().unwrap()).unwrap();
        assert_eq!(graph3::to_graph3(&back), graph3::to_graph3(&g));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn import_reports_missing_file() {
        let err = import_graph3("/nonexistent/cartographer_no_such_file.graph3").unwrap_err();
        assert!(err.contains("failed to read"), "got: {err}");
    }

    #[test]
    fn import_reports_parse_errors_with_line_numbers() {
        let path = tmp("bad");
        std::fs::write(&path, "1 2\n3 4 5 6\n").unwrap();
        let err = import_graph3(path.to_str().unwrap()).unwrap_err();
        assert!(err.contains("line 2"), "got: {err}");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn export_writes_no_trailing_blank_line() {
        let g = graph3::from_graph3("1 2\n").unwrap();
        let path = tmp("nl");
        export_graph3(&g, path.to_str().unwrap()).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text, "0 : NC\n1 : NC\n0 1\n");
        std::fs::remove_file(&path).ok();
    }
}
