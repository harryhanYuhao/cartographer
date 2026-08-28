//! Graph3 (.graph3) format: parsing and rendering of colored multigraphs.
//!
//! Graph3 is a line-based, plain-text encoding of a multigraph (parallel
//! edges and self-loops allowed) in which every line declares one or two
//! vertices and, when two are named, a number of parallel edges between
//! them; an optional `: type` suffix annotates the line's vertex (one hex
//! token) or its edges (two or three hex tokens).
//!
//! # Lexical rules
//!
//! - Lines are separated by `\n` or `\r\n`; a lone `\r` counts as ordinary
//!   whitespace. Everything from `#` to the end of the line is a comment.
//!   Whitespace-only lines are ignored.
//! - A declaration is `numbers [ ':' type ]` where `numbers` is one, two, or
//!   three whitespace-separated hex tokens (`[0-9a-fA-F]+`, no `0x` prefix,
//!   no sign) and `type` is a single whitespace-free token, matched
//!   case-insensitively. Whitespace around the `:` is free; the `:` splits
//!   the line at its first occurrence.
//! - Vertex labels are numeric: case-insensitive digits, leading zeros
//!   ignored, canonical form lowercase without leading zeros. The vertex set
//!   is exactly the set of labels mentioned in the file.
//!
//! # Types
//!
//! - Vertex types (one-token lines): `NC` (default), `H` (Hadamard node),
//!   `Z(n)`, `X(n)` — a spider with phase `n * pi / 4` where `n` is a plain
//!   decimal integer (no sign, leading zeros allowed) reduced mod 8.
//! - Edge types (two/three-token lines): `NC` (default), `H` (Hadamard).
//!
//! # Line semantics and resolution
//!
//! | Tokens | Meaning |
//! | --- | --- |
//! | `a` | Declare vertex `a`. |
//! | `a b` | Declare `a`, `b` and one `NC` edge between them. |
//! | `a b c` | Declare `a`, `b` and `c` parallel `NC` edges. |
//! | `a : t` | Declare `a` and set its type to `t`. |
//! | `a b : t` | Declare `a`, `b` and one edge of type `t`. |
//! | `a b c : t` | Declare `a`, `b` and `c` parallel edges of type `t`. |
//!
//! `c` is hex; `c = 0` declares the endpoints without an edge. Self-loops
//! are allowed. Edge endpoints are unordered, and multiplicities are summed
//! per (pair, type); different types on the same pair coexist. The last
//! vertex-type declaration wins; an untyped mention never resets a type, and
//! changing a type never removes edges. The whole file is rejected, with the
//! offending 1-based line number in the error, on malformed syntax, an
//! unknown type string, a per-line multiplicity above 10 000, or a pair
//! whose total multiplicity across all lines and types exceeds 10 000.
//!
//! # Rendering
//!
//! [`to_graph3`] writes the alive induced subgraph with vertices remapped to
//! dense labels `0..n` in ascending index order: one `a : TYPE` line for
//! every alive vertex (including `NC` ones), then one line per (pair, type)
//! edge group sorted by pair with `NC` before `H`; the multiplicity is
//! emitted as a third hex token only when greater than 1. The output is
//! canonical, so parsing it again yields an equivalent graph.

use std::collections::HashMap;
use std::path::Path;

use petgraph::graph::NodeIndex;

use crate::graph::{EColor, Graph, VColor};

/// Parser safety ceiling for edge multiplicities, per line and per pair.
const MAX_MULTIPLICITY: u32 = 10_000;

/// Render the alive induced subgraph of `g` as Graph3 text (see the module
/// docs for the exact layout).
pub fn to_graph3(g: &Graph) -> String {
    let alive: Vec<NodeIndex> = g.alive_vertices().collect();
    let mut remap: HashMap<usize, usize> = HashMap::new();
    for (i, &v) in alive.iter().enumerate() {
        remap.insert(v.index(), i);
    }

    // Group alive edges by unordered endpoint pair and color.
    let mut groups: HashMap<(usize, usize, bool), u32> = HashMap::new();
    for (s, t, e) in g.edges() {
        let mut a = remap[&s.index()];
        let mut b = remap[&t.index()];
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        let is_h = g.edge_color(e) == EColor::H;
        *groups.entry((a, b, is_h)).or_insert(0) += 1;
    }
    let mut edges: Vec<(usize, usize, bool, u32)> = groups
        .into_iter()
        .map(|((a, b, is_h), m)| (a, b, is_h, m))
        .collect();
    // false < true, so the NC group of a pair precedes its H group.
    edges.sort_by_key(|&(a, b, is_h, _)| (a, b, is_h));

    let mut out = String::new();
    for (i, &v) in alive.iter().enumerate() {
        out.push_str(&format!("{i:x} : {}\n", type_string(g.label(v))));
    }
    for (a, b, is_h, m) in edges {
        let suffix = match (m > 1, is_h) {
            (false, false) => String::new(),
            (false, true) => " : H".to_string(),
            (true, false) => format!(" {m:x}"),
            (true, true) => format!(" {m:x} : H"),
        };
        out.push_str(&format!("{a:x} {b:x}{suffix}\n"));
    }
    out
}

/// Write `g` to a Graph3 file (see [`to_graph3`]).
pub fn to_graph3_file(g: &Graph, path: impl AsRef<Path>) -> std::io::Result<()> {
    std::fs::write(path.as_ref(), to_graph3(g))
}

/// The canonical Graph3 spelling of a vertex color.
fn type_string(c: VColor) -> String {
    match c {
        VColor::Z(s) => format!("Z({})", s % 8),
        VColor::X(s) => format!("X({})", s % 8),
        VColor::H => "H".to_string(),
        VColor::NC => "NC".to_string(),
    }
}

/// Parse Graph3 text into a colored multigraph (see the module docs).
///
/// On error the returned string carries the 1-based line number of the first
/// offending line.
pub fn from_graph3(input: &str) -> Result<Graph, String> {
    // First-appearance label registry.
    let mut labels: Vec<String> = Vec::new();
    let mut ids: HashMap<String, usize> = HashMap::new();
    // Last vertex-type declaration per label id.
    let mut vtypes: HashMap<usize, VColor> = HashMap::new();
    // Parallel-edge count per (ordered pair, is_h).
    let mut groups: HashMap<(usize, usize, bool), u32> = HashMap::new();
    // Total multiplicity per unordered pair, summed across all types.
    let mut pair_total: HashMap<(usize, usize), u32> = HashMap::new();

    for (i, raw) in input.lines().enumerate() {
        let line_no = i + 1;
        // Strip the comment, then surrounding whitespace (a lone \r is
        // ordinary whitespace, so \r\n and stray \r both wash out here).
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        // Split at the first ':' into the numbers part and an optional type
        // token; the type must be exactly one whitespace-free token.
        let (nums, ty) = match line.find(':') {
            Some(pos) => (&line[..pos], Some(&line[pos + 1..])),
            None => (line, None),
        };
        let ty = match ty {
            Some(t) => {
                let t = t.trim();
                if t.is_empty() || t.split_whitespace().count() != 1 {
                    return Err(format!(
                        "graph3 line {line_no}: type suffix must be one whitespace-free token"
                    ));
                }
                Some(t.to_ascii_lowercase())
            }
            None => None,
        };

        let tokens: Vec<&str> = nums.split_whitespace().collect();
        let n = tokens.len();
        if n == 0 || n > 3 {
            return Err(format!(
                "graph3 line {line_no}: expected 1, 2, or 3 hex tokens, found {n}"
            ));
        }

        if n == 1 {
            // Vertex declaration, optionally typed; last declaration wins.
            let a = canonical_hex(tokens[0], line_no)?;
            let ia = register_label(&mut labels, &mut ids, a);
            if let Some(t) = ty {
                match parse_vertex_type(&t) {
                    Some(c) => {
                        vtypes.insert(ia, c);
                    }
                    None => {
                        return Err(format!("graph3 line {line_no}: unknown vertex type '{t}'"));
                    }
                }
            }
        } else {
            // Edge declaration: a b [c] [: t]. The type is validated even
            // when the multiplicity is zero.
            let a = canonical_hex(tokens[0], line_no)?;
            let b = canonical_hex(tokens[1], line_no)?;
            let ia = register_label(&mut labels, &mut ids, a);
            let ib = register_label(&mut labels, &mut ids, b);
            let m = if n == 3 {
                graph3_multiplicity(tokens[2], line_no)?
            } else {
                1
            };
            let is_h = match ty.as_deref() {
                None => false,
                Some("h") => true,
                Some("nc") => false,
                Some(other) => {
                    return Err(format!(
                        "graph3 line {line_no}: unknown edge type '{other}'"
                    ));
                }
            };
            if m > 0 {
                let key = order_pair(ia, ib);
                let total = pair_total.entry(key).or_insert(0);
                *total += m;
                if *total > MAX_MULTIPLICITY {
                    return Err(format!(
                        "graph3 line {line_no}: pair edges exceed {MAX_MULTIPLICITY}"
                    ));
                }
                *groups.entry((key.0, key.1, is_h)).or_insert(0) += m;
            }
        }
    }

    // Assign dense ids in ascending numeric-label order.
    let mut order: Vec<usize> = (0..labels.len()).collect();
    order.sort_by_key(|&i| numeric_label_key(&labels[i]));
    let mut remap = vec![0usize; labels.len()];
    for (new, &old) in order.iter().enumerate() {
        remap[old] = new;
    }

    let mut edges: Vec<(usize, usize, bool, u32)> = groups
        .into_iter()
        .map(|((a, b, is_h), m)| (remap[a], remap[b], is_h, m))
        .collect();
    edges.sort_by_key(|&(a, b, is_h, _)| (a, b, is_h));

    let mut g = Graph::new();
    for &old in &order {
        let color = vtypes.get(&old).copied().unwrap_or(VColor::NC);
        g.add_vertex_with(color);
    }
    for (a, b, is_h, m) in edges {
        let c = if is_h { EColor::H } else { EColor::NC };
        for _ in 0..m {
            g.add_edge_c(NodeIndex::new(a), NodeIndex::new(b), c);
        }
    }
    Ok(g)
}

/// Read and parse a Graph3 file (see [`from_graph3`]).
pub fn from_graph3_file(path: impl AsRef<Path>) -> Result<Graph, String> {
    let text = std::fs::read_to_string(path.as_ref())
        .map_err(|e| format!("failed to read graph3 file: {e}"))?;
    from_graph3(&text)
}

/// Parse a vertex type string (already lowercased): `nc`, `h`, `z(n)`, or
/// `x(n)` with `n` a non-negative decimal integer reduced mod 8 (folded
/// digit-wise, so arbitrary digit counts cannot overflow).
fn parse_vertex_type(t: &str) -> Option<VColor> {
    match t {
        "nc" => Some(VColor::NC),
        "h" => Some(VColor::H),
        _ => {
            let (is_z, digits) = t
                .strip_prefix("z(")
                .map(|d| (true, d))
                .or_else(|| t.strip_prefix("x(").map(|d| (false, d)))?;
            let digits = digits.strip_suffix(')')?;
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let p = phase_mod8(digits);
            Some(if is_z { VColor::Z(p) } else { VColor::X(p) })
        }
    }
}

/// Fold decimal digits into a phase mod 8.
fn phase_mod8(digits: &str) -> u8 {
    digits
        .bytes()
        .fold(0u8, |acc, b| (acc * 10 + (b - b'0')) % 8)
}

/// Validate a Graph3 token is bare hex and return its canonical (lowercase,
/// no leading zeros) form.
fn canonical_hex(token: &str, line: usize) -> Result<String, String> {
    if token.is_empty() || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("graph3 line {line}: invalid hex token '{token}'"));
    }
    let lower = token.to_ascii_lowercase();
    let trimmed = lower.trim_start_matches('0');
    Ok(if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    })
}

/// Parse a Graph3 multiplicity token, enforcing the 10 000 ceiling.
fn graph3_multiplicity(token: &str, line: usize) -> Result<u32, String> {
    let canon = canonical_hex(token, line)?;
    if canon.len() > 4 {
        return Err(format!(
            "graph3 line {line}: multiplicity '{token}' exceeds {MAX_MULTIPLICITY}"
        ));
    }
    let m = u32::from_str_radix(&canon, 16)
        .map_err(|_| format!("graph3 line {line}: invalid multiplicity '{token}'"))?;
    if m > MAX_MULTIPLICITY {
        return Err(format!(
            "graph3 line {line}: multiplicity '{token}' exceeds {MAX_MULTIPLICITY}"
        ));
    }
    Ok(m)
}

/// Intern a canonical label, returning its vertex id (first-appearance order).
fn register_label(
    labels: &mut Vec<String>,
    ids: &mut HashMap<String, usize>,
    label: String,
) -> usize {
    if let Some(&i) = ids.get(&label) {
        return i;
    }
    let i = labels.len();
    labels.push(label.clone());
    ids.insert(label, i);
    i
}

/// Canonical unordered pair key.
fn order_pair(a: usize, b: usize) -> (usize, usize) {
    if a <= b { (a, b) } else { (b, a) }
}

/// Numeric sort key for a canonical label: shorter (fewer hex digits) is
/// smaller; equal length compares lexicographically.
fn numeric_label_key(label: &str) -> (usize, &str) {
    (label.len(), label)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sorted `(u, v, color)` triples of the alive edges, u <= v.
    fn sorted_colored_edges(g: &Graph) -> Vec<(usize, usize, EColor)> {
        let mut out: Vec<_> = g
            .edges()
            .map(|(s, t, e)| {
                let (x, y) = (s.index(), t.index());
                (x.min(y), x.max(y), g.edge_color(e))
            })
            .collect();
        out.sort_by_key(|&(x, y, c)| (x, y, c == EColor::H));
        out
    }

    #[test]
    fn spec_example() {
        let g = from_graph3("1 2 2\n2 3 : H\n1 : Z(0)\n1 3\n3\n").unwrap();
        assert_eq!(g.node_count(), 3);
        // Two NC edges 1-2, one H edge 2-3, one NC edge 1-3 (labels 1,2,3
        // remap to dense 0,1,2).
        assert_eq!(
            sorted_colored_edges(&g),
            vec![
                (0, 1, EColor::NC),
                (0, 1, EColor::NC),
                (0, 2, EColor::NC),
                (1, 2, EColor::H),
            ]
        );
        assert_eq!(g.label(NodeIndex::new(0)), VColor::Z(0));
        assert_eq!(g.label(NodeIndex::new(1)), VColor::NC);
        assert_eq!(g.label(NodeIndex::new(2)), VColor::NC);
    }

    #[test]
    fn round_trip_and_reparse() {
        let src = r"1 2 2
2 3
1
1 3
a a f
10 20 3
";
        let g = from_graph3(src).unwrap();
        assert_eq!(g.node_count(), 6);
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)), 2); // 1-2
        assert_eq!(g.edge_multiplicity(NodeIndex::new(1), NodeIndex::new(2)), 1); // 2-3
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(2)), 1); // 1-3
        assert_eq!(
            g.edge_multiplicity(NodeIndex::new(3), NodeIndex::new(3)),
            15
        ); // a-a
        assert_eq!(g.edge_multiplicity(NodeIndex::new(4), NodeIndex::new(5)), 3); // 10-20
        let out = to_graph3(&g);
        assert_eq!(out, from_graph3(&out).unwrap().to_graph3());
    }

    #[test]
    fn repeated_pairs_sum_per_type() {
        // `b a` adds one more edge to the unordered pair: 3 + 1 = 4.
        let g = from_graph3("1 2 3\n2 1\n").unwrap();
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)), 4);
    }

    #[test]
    fn isolated_vertex_and_zero_multiplicity() {
        let g = from_graph3("f\n1 2 0\n").unwrap();
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edge_count(), 0);
        let out = to_graph3(&g);
        // Three annotated vertices, no edges.
        assert_eq!(out, "0 : NC\n1 : NC\n2 : NC\n");
    }

    #[test]
    fn canonicalizes_labels() {
        // a, A, 0a, 00a all spell hex 10, so they are one vertex.
        let g = from_graph3("a\nA\n0a\n00a\n").unwrap();
        assert_eq!(g.node_count(), 1);

        // A == a and 0B == b: multiplicities sum per pair: 2 + 3 = 5.
        let g2 = from_graph3("A 0B 2\n00a b 3\n").unwrap();
        assert_eq!(g2.node_count(), 2);
        assert_eq!(
            g2.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)),
            5
        );
    }

    #[test]
    fn orders_vertices_numerically() {
        // Labels 2, f(15), 10(16). Numeric order is 2 < f < 10, so 10 is node 2
        // even though "10" sorts before "2" as text.
        let g = from_graph3("10 2\nf\n").unwrap();
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(2)), 1);
        assert_eq!(g.edge_multiplicity(NodeIndex::new(1), NodeIndex::new(2)), 0);
    }

    #[test]
    fn ignores_blank_lines_and_whitespace() {
        let g = from_graph3("\n1 2\n   \n2 3\n").unwrap();
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)), 1);
        assert_eq!(g.edge_multiplicity(NodeIndex::new(1), NodeIndex::new(2)), 1);
    }

    #[test]
    fn handles_tabs() {
        let g = from_graph3("1\t2\t3\n").unwrap();
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)), 3);
    }

    #[test]
    fn comments_and_crlf() {
        // Full-line and trailing comments, \r\n separators, and a lone \r
        // acting as ordinary whitespace between tokens.
        let g = from_graph3("# header\r\n1 2 # edge\r\n1\r3\r\n").unwrap();
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)), 1);
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(2)), 1);
    }

    #[test]
    fn self_loop_does_not_affect_degree() {
        let g = from_graph3("a a\n").unwrap();
        assert_eq!(g.node_count(), 1);
        assert!(g.has_edge(NodeIndex::new(0), NodeIndex::new(0)));
        assert_eq!(g.degree(NodeIndex::new(0)), 0);
    }

    #[test]
    fn typed_self_loops() {
        let g = from_graph3("a a 2 : H\n").unwrap();
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(0)), 2);
        assert_eq!(
            sorted_colored_edges(&g),
            vec![(0, 0, EColor::H), (0, 0, EColor::H)]
        );
    }

    #[test]
    fn multiplicity_ceiling_boundaries() {
        // 10000 (0x2710) is the largest allowed multiplicity.
        let g = from_graph3("1 2 2710").unwrap();
        assert_eq!(g.edge_count(), 10_000);
        // Just over the ceiling, and values that overflow the 4-hex-digit
        // fast path.
        assert!(from_graph3("1 2 2711").is_err());
        assert!(from_graph3("1 2 ffff").is_err());
        assert!(from_graph3("1 2 100000").is_err());
        // Leading zeros are ignored, so "0000" is a zero multiplicity.
        let g0 = from_graph3("1 2 0000").unwrap();
        assert_eq!(g0.edge_count(), 0);
        assert_eq!(g0.node_count(), 2);
    }

    #[test]
    fn pair_ceiling_sums_across_types() {
        // 10000 NC edges plus 1 H edge on the same pair exceeds the ceiling.
        let e = from_graph3("1 2 2710\n1 2 1 : H\n").unwrap_err();
        assert!(e.contains("line 2"), "got: {e}");
        // The same total split across two types but under the ceiling is fine.
        let ok = from_graph3("1 2 2710\n1 2 0 : H\n").unwrap();
        assert_eq!(ok.edge_count(), 10_000);
    }

    #[test]
    fn type_parsing() {
        let g = from_graph3("1 : z(10)\n2 : X(0008)\n3 : H\n4 : nc\n5 : Z(7)\n1 2\n").unwrap();
        // Phases are decimal, reduced mod 8, case-insensitive.
        assert_eq!(g.label(NodeIndex::new(0)), VColor::Z(2)); // 10 mod 8
        assert_eq!(g.label(NodeIndex::new(1)), VColor::X(0));
        assert_eq!(g.label(NodeIndex::new(2)), VColor::H);
        assert_eq!(g.label(NodeIndex::new(3)), VColor::NC);
        assert_eq!(g.label(NodeIndex::new(4)), VColor::Z(7));
        // Arbitrarily many digits fold mod 8 without overflowing.
        let g2 = from_graph3("1 : Z(0000000000000000000000009)\n").unwrap();
        assert_eq!(g2.label(NodeIndex::new(0)), VColor::Z(1));
    }

    #[test]
    fn last_vertex_type_wins_and_mentions_do_not_reset() {
        let g = from_graph3("1 : Z(3)\n1 : X(1)\n1\n2 1\n").unwrap();
        assert_eq!(g.label(NodeIndex::new(0)), VColor::X(1));
        // The bare `1` mention did not reset the type; the edge exists.
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)), 1);
        // Changing a type never removes edges.
        let g2 = from_graph3("1 2\n1 : H\n").unwrap();
        assert_eq!(g2.label(NodeIndex::new(0)), VColor::H);
        assert_eq!(
            g2.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)),
            1
        );
    }

    #[test]
    fn mixed_types_coexist_on_a_pair() {
        let g = from_graph3("1 2 2\n1 2 : H\n1 2 3 : H\n").unwrap();
        assert_eq!(
            sorted_colored_edges(&g),
            vec![
                (0, 1, EColor::NC),
                (0, 1, EColor::NC),
                (0, 1, EColor::H),
                (0, 1, EColor::H),
                (0, 1, EColor::H),
                (0, 1, EColor::H),
            ]
        );
    }

    #[test]
    fn vertex_h_differs_from_edge_h() {
        let g = from_graph3("1 : H\n1 2 : H\n").unwrap();
        assert_eq!(g.label(NodeIndex::new(0)), VColor::H);
        assert_eq!(sorted_colored_edges(&g), vec![(0, 1, EColor::H)]);
    }

    #[test]
    fn zero_multiplicity_still_validates_the_type() {
        assert!(from_graph3("1 2 0 : H\n").is_ok());
        assert!(from_graph3("1 2 0 : QUX\n").is_err());
    }

    #[test]
    fn errors() {
        assert!(from_graph3("1 2 3 4").is_err());
        assert!(from_graph3("1 0x1").is_err());
        assert!(from_graph3("1 -1").is_err());
        assert!(from_graph3("1 : QUX").is_err()); // unknown vertex type
        assert!(from_graph3("1 2 : QUX").is_err()); // unknown edge type
        assert!(from_graph3("1 2 : Z(0)").is_err()); // Z is not an edge type
        assert!(from_graph3("1 : Z (0)").is_err()); // whitespace inside type
        assert!(from_graph3("1 : Z()").is_err());
        assert!(from_graph3("1 : Z(-1)").is_err());
        assert!(from_graph3("1 2 : H x").is_err()); // two-token type suffix
        assert!(from_graph3(": Z(0)").is_err()); // no hex token
        assert!(from_graph3("1 :").is_err()); // empty type suffix
    }

    #[test]
    fn errors_report_line_numbers() {
        let e = from_graph3("1 2\n2 3\n0x4 5\n").unwrap_err();
        assert!(e.contains("line 3"), "got: {e}");
        let e2 = from_graph3("1 2\n\n2 3 2711\n").unwrap_err();
        assert!(e2.contains("line 3"), "got: {e2}");
        let e3 = from_graph3("1 2\n3 4 : BAD\n").unwrap_err();
        assert!(e3.contains("line 2"), "got: {e3}");
    }

    #[test]
    fn empty_input() {
        assert_eq!(from_graph3("").unwrap().node_count(), 0);
        assert_eq!(from_graph3("  \n\n  ").unwrap().node_count(), 0);
    }

    #[test]
    fn export_from_constructed_graph() {
        let mut g = Graph::new();
        let _a = g.add_vertex_with(VColor::NC);
        let _b = g.add_vertex_with(VColor::NC);
        let _c = g.add_vertex_with(VColor::NC);
        g.add_edge_c(NodeIndex::new(0), NodeIndex::new(1), EColor::NC);
        g.add_edge_c(NodeIndex::new(0), NodeIndex::new(1), EColor::NC);
        g.add_edge_c(NodeIndex::new(0), NodeIndex::new(0), EColor::NC); // self-loop
        // vertex 2 is isolated
        let out = to_graph3(&g);
        assert_eq!(out, "0 : NC\n1 : NC\n2 : NC\n0 0\n0 1 2\n");
        assert_eq!(from_graph3(&out).unwrap().to_graph3(), out);
    }

    #[test]
    fn export_annotates_every_vertex_and_orders_groups() {
        // Pair (0, 1) carries both NC and H groups: NC line comes first.
        let out = from_graph3("1 2 : H\n1 2\n").unwrap().to_graph3();
        assert_eq!(out, "0 : NC\n1 : NC\n0 1\n0 1 : H\n");
    }

    #[test]
    fn colored_round_trip() {
        let mut g = Graph::new();
        let a = g.add_vertex_with(VColor::Z(3));
        let b = g.add_vertex_with(VColor::X(7));
        let c = g.add_vertex_with(VColor::H);
        g.add_edge_c(a, b, EColor::H);
        g.add_edge_c(a, b, EColor::NC);
        g.add_edge_c(a, b, EColor::NC);
        g.add_edge_c(c, c, EColor::H); // H self-loop
        g.add_edge_c(b, c, EColor::NC);

        let text = to_graph3(&g);
        assert_eq!(
            text,
            "0 : Z(3)\n1 : X(7)\n2 : H\n0 1 2\n0 1 : H\n1 2\n2 2 : H\n"
        );
        let back = from_graph3(&text).unwrap();
        assert_eq!(back.node_count(), 3);
        assert_eq!(back.label(NodeIndex::new(0)), VColor::Z(3));
        assert_eq!(back.label(NodeIndex::new(1)), VColor::X(7));
        assert_eq!(back.label(NodeIndex::new(2)), VColor::H);
        assert_eq!(sorted_colored_edges(&back), sorted_colored_edges(&g));
        // Canonical: exporting the reparsed graph is a fixpoint.
        assert_eq!(to_graph3(&back), text);
    }

    #[test]
    fn export_is_alive_induced_subgraph() {
        // Eliminate the center of a star: only leaves stay alive, and
        // elimination fills them into a clique.
        let mut g = Graph::from_edges([(0, 1), (0, 2), (0, 3)]);
        g.elim(NodeIndex::new(0));
        assert_eq!(g.alive_count(), 3);
        let out = to_graph3(&g);
        let parsed = from_graph3(&out).unwrap();
        assert_eq!(parsed.node_count(), 3);
        assert_eq!(
            parsed.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)),
            1
        );
        assert_eq!(
            parsed.edge_multiplicity(NodeIndex::new(1), NodeIndex::new(2)),
            1
        );
        assert_eq!(
            parsed.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(2)),
            1
        );
    }

    #[test]
    fn file_round_trip() {
        let path =
            std::env::temp_dir().join(format!("cartographer_graph3_{}.graph3", std::process::id()));
        let g = from_graph3("1 2 3\n2 3\nf : Z(5)\n").unwrap();
        to_graph3_file(&g, &path).unwrap();
        let g2 = from_graph3_file(&path).unwrap();
        assert_eq!(to_graph3(&g), to_graph3(&g2));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn colon_whitespace_is_free() {
        // `1:Z(0)`, `1 : Z(0)`, and `1: Z(0)` are all valid, but the type
        // itself must be one whitespace-free token.
        let g = from_graph3("1:Z(0)\n2 :X(1)\n3: H\n1 2:H\n").unwrap();
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.label(NodeIndex::new(0)), VColor::Z(0));
        assert_eq!(g.label(NodeIndex::new(1)), VColor::X(1));
        assert_eq!(g.label(NodeIndex::new(2)), VColor::H);
        assert_eq!(sorted_colored_edges(&g), vec![(0, 1, EColor::H)]);
        // Tabs around the colon wash out with the trim.
        let g2 = from_graph3("4 :\tZ(2)\t\n").unwrap();
        assert_eq!(g2.label(NodeIndex::new(0)), VColor::Z(2));
    }

    #[test]
    fn comments_strip_before_tokenizing() {
        // A comment can mask what would otherwise be too many tokens or a
        // type-looking suffix; only what precedes `#` is parsed.
        let g = from_graph3("1 2 3 # 4 5 : Z(9)\n# 9 : Z(9)\n1 # 2 3 4\n").unwrap();
        assert_eq!(g.node_count(), 2); // no vertex 9 from the comment
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)), 3);
        assert_eq!(g.label(NodeIndex::new(0)), VColor::NC);
    }

    #[test]
    fn second_colon_becomes_part_of_type() {
        // The line splits at the FIRST ':'; the rest (now containing
        // whitespace) is an invalid type token.
        assert!(from_graph3("1 : Z(0) : junk\n").is_err());
        assert!(from_graph3("1 2 : H : H\n").is_err());
    }

    #[test]
    fn zero_labels_collapse() {
        // 0, 00, 000 all denote vertex 0.
        let g = from_graph3("0\n00\n000\n0 0\n").unwrap();
        assert_eq!(g.node_count(), 1);
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(0)), 1);
    }

    #[test]
    fn vertex_set_is_exactly_mentioned_labels() {
        // Labels 1 and 3 are mentioned; vertex 2 is not implied.
        let g = from_graph3("1 3\n").unwrap();
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)), 1);
    }

    #[test]
    fn uppercase_labels_and_lowercase_types() {
        // Hex labels and type tokens are case-insensitive in both roles.
        let g = from_graph3("A B F\n1 2 : h\n2 : nc\n").unwrap();
        assert_eq!(g.node_count(), 4);
        // Numeric order 1 < 2 < 10(A) < 11(B), so dense ids are
        // 1->0, 2->1, A->2, B->3; A-B carries 0xF = 15 parallel NC edges
        // and 1-2 one lowercase-`h` Hadamard edge.
        let mut expected = vec![(2, 3, EColor::NC); 15];
        expected.insert(0, (0, 1, EColor::H));
        assert_eq!(sorted_colored_edges(&g), expected);
        assert_eq!(
            g.edge_multiplicity(NodeIndex::new(2), NodeIndex::new(3)),
            15
        );
        // Vertex "2" (dense 1) was typed `nc`.
        assert_eq!(g.label(NodeIndex::new(1)), VColor::NC);
    }

    #[test]
    fn explicit_multiplicity_one_renders_bare() {
        let out = from_graph3("1 2 1\n").unwrap().to_graph3();
        assert_eq!(out, "0 : NC\n1 : NC\n0 1\n");
    }

    #[test]
    fn hex_multiplicity_and_large_labels_render() {
        // 16 parallel edges render multiplicities in hex (0x10), and labels
        // of any hex length are first-class vertices.
        let g = from_graph3("1 2 10\n3 ffffffffffff\n").unwrap();
        assert_eq!(g.node_count(), 4);
        // Dense ids: 1->0, 2->1, 3->2, ffffffffffff->3.
        assert_eq!(
            g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)),
            16
        );
        assert_eq!(g.edge_multiplicity(NodeIndex::new(2), NodeIndex::new(3)), 1);
        let out = to_graph3(&g);
        assert_eq!(out, "0 : NC\n1 : NC\n2 : NC\n3 : NC\n0 1 10\n2 3\n");
    }

    #[test]
    fn pair_ceiling_is_per_pair() {
        // 10000 edges on each of two distinct pairs: fine, the ceiling is
        // per pair, not global.
        let g = from_graph3("1 2 2710\n3 4 2710\n").unwrap();
        assert_eq!(g.edge_count(), 20_000);
    }

    #[test]
    fn multi_line_sums_may_reach_ceiling_exactly() {
        // 0x1388 = 5000 twice: exactly 10000 on the pair, still allowed.
        let g = from_graph3("1 2 1388\n1 2 1388\n").unwrap();
        assert_eq!(
            g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)),
            10_000
        );
        // One more single edge tips it over.
        assert!(from_graph3("1 2 1388\n1 2 1388\n1 2\n").is_err());
    }

    #[test]
    fn out_of_range_phases_reduce_on_export() {
        // Phases constructed out of range (only possible programmatically)
        // are reduced mod 8 when rendered.
        let mut g = Graph::new();
        g.add_vertex_with(VColor::Z(9));
        g.add_vertex_with(VColor::X(8));
        let out = to_graph3(&g);
        assert_eq!(out, "0 : Z(1)\n1 : X(0)\n");
        let back = from_graph3(&out).unwrap();
        assert_eq!(back.label(NodeIndex::new(0)), VColor::Z(1));
        assert_eq!(back.label(NodeIndex::new(1)), VColor::X(0));
    }

    #[test]
    fn empty_and_dead_graphs_render_nothing() {
        assert_eq!(to_graph3(&Graph::new()), "");
        // An empty file parses back to an empty graph.
        assert_eq!(from_graph3("").unwrap().node_count(), 0);
        // All vertices dead: the alive induced subgraph is empty.
        let mut g = Graph::new();
        let a = g.add_vertex_with(VColor::Z(1));
        let b = g.add_vertex_with(VColor::Z(2));
        g.add_edge_c(a, b, EColor::H);
        g.remove_vertex(a);
        g.remove_vertex(b);
        assert_eq!(to_graph3(&g), "");
    }

    #[test]
    fn export_skips_dead_vertices_and_relabels() {
        // Killing the middle vertex leaves a and c alive; they are remapped
        // to dense labels 0 and 1.
        let mut g = Graph::new();
        let a = g.add_vertex_with(VColor::Z(2));
        let b = g.add_vertex_with(VColor::X(5));
        let c = g.add_vertex_with(VColor::H);
        g.add_edge_c(a, b, EColor::H);
        g.add_edge_c(a, c, EColor::H);
        g.remove_vertex(b);
        let out = to_graph3(&g);
        assert_eq!(out, "0 : Z(2)\n1 : H\n0 1 : H\n");
    }

    #[test]
    fn random_colored_graphs_round_trip() {
        use crate::generator::zx::rand_graph_like_zx;
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let mut rng = StdRng::seed_from_u64(0xC0FFEE);
        for (n, e) in [(3usize, 0usize), (5, 4), (8, 12), (12, 20), (16, 40)] {
            let mut g = rand_graph_like_zx(n, e, &mut rng);
            // Sprinkle mixed vertex types with in-range phases...
            let verts: Vec<_> = g.alive_vertices().collect();
            for (i, v) in verts.into_iter().enumerate() {
                g.set_color(
                    v,
                    match i % 4 {
                        0 => VColor::Z((i * 3 % 8) as u8),
                        1 => VColor::X(7),
                        2 => VColor::H,
                        _ => VColor::NC,
                    },
                );
            }
            // ...and flip every third edge to normal, so both edge groups
            // occur on shared pairs.
            let eids: Vec<_> = g.edges().map(|(_, _, ed)| ed).collect();
            for (i, ed) in eids.iter().enumerate() {
                if i % 3 == 0 {
                    g.set_edge_color(*ed, EColor::NC);
                }
            }

            let text = to_graph3(&g);
            let back = from_graph3(&text).unwrap();
            assert_eq!(to_graph3(&back), text, "fixpoint failed for n={n} e={e}");
            assert_eq!(back.node_count(), n);
            for i in 0..n {
                assert_eq!(
                    back.label(NodeIndex::new(i)),
                    g.label(NodeIndex::new(i)),
                    "label {i} differs for n={n} e={e}"
                );
            }
            assert_eq!(
                sorted_colored_edges(&back),
                sorted_colored_edges(&g),
                "edges differ for n={n} e={e}"
            );
        }
    }
}
