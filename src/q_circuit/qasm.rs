//! Export a [`QCircuit`] as an [OpenQASM 2.0] program.
//!
//! The output targets Qiskit's `QuantumCircuit::from_qasm_str` /
//! `from_qasm_file`: OpenQASM 2.0 with the standard `qelib1.inc` include,
//! which every Qiskit build parses.
//!
//! Gate mapping (spider phases are `s * pi/4`, mod 8):
//!
//! | `QGate`               | qelib1 statement                              |
//! |-----------------------|-----------------------------------------------|
//! | `H`                   | `h q[i];`                                     |
//! | `T` (= Z(1))          | `t q[i];`                                     |
//! | `Z(1) / Z(2) / Z(4)`  | `t` / `s` / `z`                               |
//! | `Z(6) / Z(7)`         | `sdg` / `tdg`                                 |
//! | `Z(s)`, other `s`     | `u1(s*pi/4) q[i];`                            |
//! | `X(4)`                | `x q[i];`                                     |
//! | `X(s)`, other `s`     | `rx(s*pi/4) q[i];`                            |
//! | `CNOT_C` + `CNOT_E`   | `cx q[c], q[e];`                              |
//! | `SWAP_1` + `SWAP_2`   | `swap q[a], q[b];`                            |
//! | `Input` / `Output` / `ID` | nothing (boundary markers / identity)     |
//!
//! A Z spider with phase `s*pi/4` is exactly the phase gate
//! `diag(1, e^{i*s*pi/4}) = u1(s*pi/4)`; an X spider equals `rx(s*pi/4)` up
//! to a global phase, which Qiskit's `Operator(...).equiv` ignores. Like
//! `to_graph`, at most one CNOT and one SWAP is read per layer (the last
//! pair of markers wins), and single-qubit gates on the remaining positions
//! are emitted in ascending qubit order.
//!
//! [OpenQASM 2.0]: https://openqasm.com/

use std::fmt::Write as _;
use std::path::Path;

use crate::q_circuit::def::{QCircuit, QGate};

impl QCircuit {
    /// Render the circuit as an OpenQASM 2.0 program (see the module docs
    /// for the gate mapping).
    pub fn to_qasm2(&self) -> String {
        let mut out = String::from("OPENQASM 2.0;\ninclude \"qelib1.inc\";\n");
        let _ = writeln!(out, "qreg q[{}];", self.n_qubits());

        for layer in self.layers() {
            // Two-qubit markers are collected first and emitted after the
            // single-qubit gates; like to_graph, the last pair wins.
            let (mut ctrl, mut targ) = (None, None);
            let (mut swap_a, mut swap_b) = (None, None);
            for (i, gate) in layer.iter().enumerate() {
                match gate {
                    QGate::H => {
                        let _ = writeln!(out, "h q[{i}];");
                    }
                    QGate::T => {
                        let _ = writeln!(out, "t q[{i}];");
                    }
                    QGate::Z(s) => {
                        let _ = writeln!(out, "{} q[{i}];", z_gate_name(*s));
                    }
                    QGate::X(s) => {
                        let _ = writeln!(out, "{} q[{i}];", x_gate_name(*s));
                    }
                    QGate::CNOT_C => ctrl = Some(i),
                    QGate::CNOT_E => targ = Some(i),
                    QGate::SWAP_1 => swap_a = Some(i),
                    QGate::SWAP_2 => swap_b = Some(i),
                    // Boundary markers and the identity emit nothing.
                    QGate::Input | QGate::Output | QGate::ID => {}
                }
            }
            if let (Some(c), Some(t)) = (ctrl, targ) {
                let _ = writeln!(out, "cx q[{c}], q[{t}];");
            }
            if let (Some(a), Some(b)) = (swap_a, swap_b) {
                let _ = writeln!(out, "swap q[{a}], q[{b}];");
            }
        }
        out
    }

    /// Write the OpenQASM 2.0 program of this circuit to `path`.
    pub fn to_qasm2_file(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        std::fs::write(path, self.to_qasm2())
    }
}

/// qelib1 gate (with argument) for a Z spider of phase `s * pi/4`: the
/// standard named gates where they exist, `u1` otherwise.
fn z_gate_name(s: u8) -> String {
    match s % 8 {
        0 => "u1(0)".to_string(),
        1 => "t".to_string(),
        2 => "s".to_string(),
        4 => "z".to_string(),
        6 => "sdg".to_string(),
        7 => "tdg".to_string(),
        k => format!("u1({k}*pi/4)"),
    }
}

/// qelib1 gate (with argument) for an X spider of phase `s * pi/4`: `x` at
/// phase pi, `rx` otherwise.
fn x_gate_name(s: u8) -> String {
    match s % 8 {
        0 => "rx(0)".to_string(),
        4 => "x".to_string(),
        k => format!("rx({k}*pi/4)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A circuit assembled from explicit layers.
    fn circuit(n: usize, layers: Vec<Vec<QGate>>) -> QCircuit {
        let mut c = QCircuit::emtpy(n);
        for layer in layers {
            c.push_layer(layer);
        }
        c
    }

    #[test]
    fn empty_circuit_is_header_only() {
        assert_eq!(
            QCircuit::emtpy(3).to_qasm2(),
            "OPENQASM 2.0;\ninclude \"qelib1.inc\";\nqreg q[3];\n"
        );
    }

    #[test]
    fn exports_single_and_two_qubit_gates() {
        let c = circuit(
            2,
            vec![
                vec![QGate::H, QGate::T],
                vec![QGate::CNOT_C, QGate::CNOT_E],
                vec![QGate::SWAP_1, QGate::SWAP_2],
                vec![QGate::Z(5), QGate::X(6)],
            ],
        );
        assert_eq!(
            c.to_qasm2(),
            concat!(
                "OPENQASM 2.0;\n",
                "include \"qelib1.inc\";\n",
                "qreg q[2];\n",
                "h q[0];\n",
                "t q[1];\n",
                "cx q[0], q[1];\n",
                "swap q[0], q[1];\n",
                "u1(5*pi/4) q[0];\n",
                "rx(6*pi/4) q[1];\n",
            )
        );
    }

    #[test]
    fn maps_spider_phases_to_named_gates() {
        // One Z(s) layer (then one X(s) layer) per phase s in 0..8 on a
        // single qubit: standard angles map to named gates, the rest to
        // u1/rx with an explicit s*pi/4 argument.
        let mut z_layers = vec![];
        let mut x_layers = vec![];
        for s in 0u8..8 {
            z_layers.push(vec![QGate::Z(s)]);
            x_layers.push(vec![QGate::X(s)]);
        }

        let zq = circuit(1, z_layers).to_qasm2();
        let body: Vec<&str> = zq.lines().skip(3).collect();
        assert_eq!(
            body,
            vec![
                "u1(0) q[0];",
                "t q[0];",
                "s q[0];",
                "u1(3*pi/4) q[0];",
                "z q[0];",
                "u1(5*pi/4) q[0];",
                "sdg q[0];",
                "tdg q[0];",
            ]
        );

        let xq = circuit(1, x_layers).to_qasm2();
        let body: Vec<&str> = xq.lines().skip(3).collect();
        assert_eq!(
            body,
            vec![
                "rx(0) q[0];",
                "rx(1*pi/4) q[0];",
                "rx(2*pi/4) q[0];",
                "rx(3*pi/4) q[0];",
                "x q[0];",
                "rx(5*pi/4) q[0];",
                "rx(6*pi/4) q[0];",
                "rx(7*pi/4) q[0];",
            ]
        );
    }

    #[test]
    fn skips_boundary_and_identity_gates() {
        let c = circuit(
            2,
            vec![
                vec![QGate::Input, QGate::ID],
                vec![QGate::ID, QGate::Output],
            ],
        );
        assert_eq!(
            c.to_qasm2(),
            "OPENQASM 2.0;\ninclude \"qelib1.inc\";\nqreg q[2];\n"
        );
    }

    #[test]
    fn unpaired_two_qubit_markers_emit_nothing() {
        // Mirrors to_graph: a CNOT/SWAP is only wired when both markers are
        // present in the layer.
        let c = circuit(2, vec![vec![QGate::CNOT_C, QGate::H]]);
        let qasm = c.to_qasm2();
        assert!(qasm.contains("h q[1];"));
        assert!(!qasm.contains("cx"));

        let c = circuit(2, vec![vec![QGate::SWAP_2, QGate::ID]]);
        assert!(!c.to_qasm2().contains("swap"));
    }

    #[test]
    fn file_roundtrip() {
        let c = circuit(
            2,
            vec![vec![QGate::H, QGate::T], vec![QGate::CNOT_C, QGate::CNOT_E]],
        );
        let path = std::env::temp_dir().join("cartographer_qasm2_roundtrip.qasm");
        c.to_qasm2_file(&path).unwrap();
        let read_back = std::fs::read_to_string(&path).unwrap();
        assert_eq!(read_back, c.to_qasm2());
    }

    #[test]
    fn random_circuit_exports_header_and_lines() {
        let mut rng = rand::rng();
        let c = QCircuit::rand_circuit(4, 8, &mut rng);
        let qasm = c.to_qasm2();
        assert!(qasm.starts_with("OPENQASM 2.0;\n"));
        assert!(qasm.contains("include \"qelib1.inc\";\n"));
        assert!(qasm.contains("qreg q[4];\n"));
        for line in qasm.lines().skip(3) {
            assert!(line.ends_with(';'), "unterminated line: {line}");
            assert!(!line.is_empty());
        }
    }
}
