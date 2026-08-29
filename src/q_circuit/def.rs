use std::iter::Enumerate;

use crate::{
    graph::{Graph, VColor},
    q_circuit::def::QGate::ID,
};
use petgraph::graph::NodeIndex;

use rand::Rng;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum QGate {
    Input,
    Output,
    H,
    X(u8), // X(s) denmote X spider with phase s * \pi/4
    Z(u8), // Z(s) denmote Z spider with phase s * \pi/4
    T,
    CNOT_C, // Control of CNOT
    CNOT_E, // Target of CNOT
    SWAP_1, // Position 1 of SWAP
    SWAP_2, // Position 2 of SWAP
    ID,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct QCircuit {
    n_qubits: usize,
    gates: Vec<Vec<QGate>>, // gate[i] is the nth layer of gate
}

impl QCircuit {
    pub fn emtpy(n: usize) -> Self {
        QCircuit {
            n_qubits: n,
            gates: vec![],
        }
    }

    /// Number of qubits the circuit acts on.
    pub fn n_qubits(&self) -> usize {
        self.n_qubits
    }

    /// The gate layers in application order: `layers()[t][i]` is the gate on
    /// qubit `i` at time step `t` (each layer has `n_qubits` entries).
    pub fn layers(&self) -> &[Vec<QGate>] {
        &self.gates
    }

    /// Append a gate layer; its length must equal `n_qubits`.
    pub fn push_layer(&mut self, layer: Vec<QGate>) {
        assert_eq!(layer.len(), self.n_qubits);
        self.gates.push(layer);
    }

    fn rand_layer(n: usize, rng: &mut impl Rng) -> Vec<QGate> {
        let mut layer = vec![];
        let pos = rng.random_range(0..n);
        let gate: QGate;
        if n >= 2 {
            gate = match rng.random_range(0..=5) {
                0 => QGate::H,
                1 => QGate::X(rng.random_range(0..8)),
                2 => QGate::Z(rng.random_range(0..8)),
                3 => QGate::T,
                4 => QGate::CNOT_C,
                5 => QGate::SWAP_1,
                _ => unreachable!(),
            };
        } else {
            gate = match rng.random_range(0..=3) {
                0 => QGate::H,
                1 => QGate::X(rng.random_range(0..8)),
                2 => QGate::Z(rng.random_range(0..8)),
                3 => QGate::T,
                _ => unreachable!(),
            }
        }

        if gate == QGate::CNOT_C {
            let mut pos2 = rng.random_range(0..n);
            while pos2 == pos {
                pos2 = rng.random_range(0..n);
            }
            for i in 0..n {
                if i == pos {
                    layer.push(QGate::CNOT_C);
                } else if i == pos2 {
                    layer.push(QGate::CNOT_E)
                } else {
                    layer.push(QGate::ID)
                }
            }
        } else if gate == QGate::SWAP_1 {
            let mut pos2 = rng.random_range(0..n);
            while pos2 == pos {
                pos2 = rng.random_range(0..n);
            }
            for i in 0..n {
                if i == pos {
                    layer.push(QGate::SWAP_1);
                } else if i == pos2 {
                    layer.push(QGate::SWAP_2);
                } else {
                    layer.push(QGate::ID);
                }
            }
        } else {
            for i in 0..n {
                if i == pos {
                    layer.push(gate.clone());
                } else {
                    layer.push(QGate::ID);
                }
            }
        }

        layer
    }

    pub fn rand_circuit(n_qubit: usize, depth: usize, rng: &mut impl Rng) -> Self {
        let mut circuit = QCircuit::emtpy(n_qubit);
        for _ in 0..depth {
            circuit.gates.push(QCircuit::rand_layer(n_qubit, rng));
        }
        circuit
    }

    /// Starts and end with all X(0)
    pub fn to_graph(&self) -> Graph {
        let mut g = Graph::new();
        let mut pre_nod_idx: Vec<NodeIndex> = vec![];
        for (index, layer) in self.gates.iter().enumerate() {
            assert_eq!(layer.len(), self.n_qubits);
            let mut cur_node_idx: Vec<NodeIndex> = vec![];
            let mut cnot: (Option<NodeIndex>, Option<NodeIndex>) = (None, None);
            let mut swap: (Option<usize>, Option<usize>) = (None, None);

            for (i, j) in layer.iter().enumerate() {
                match j {
                    &QGate::H => cur_node_idx.push(g.add_vertex_with(VColor::H)),
                    &QGate::X(s) => cur_node_idx.push(g.add_vertex_with(VColor::X(s))),
                    &QGate::Z(s) => cur_node_idx.push(g.add_vertex_with(VColor::Z(s))),
                    &QGate::T => cur_node_idx.push(g.add_vertex_with(VColor::Z(1))),
                    &QGate::Input | &QGate::Output => {
                        cur_node_idx.push(g.add_vertex_with(VColor::NC))
                    }
                    &QGate::ID => cur_node_idx.push(g.add_vertex_with(VColor::Z(0))),
                    &QGate::CNOT_C => {
                        let cur_idx = g.add_vertex_with(VColor::Z(0));
                        cur_node_idx.push(cur_idx);
                        cnot.0 = Some(cur_idx);
                    }
                    &QGate::CNOT_E => {
                        let cur_idx = g.add_vertex_with(VColor::X(0));
                        cur_node_idx.push(cur_idx);
                        cnot.1 = Some(cur_idx);
                    }
                    &QGate::SWAP_1 => {
                        let cur_idx = g.add_vertex_with(VColor::Z(0));
                        cur_node_idx.push(cur_idx);
                        swap.0 = Some(i);
                    }
                    &QGate::SWAP_2 => {
                        let cur_idx = g.add_vertex_with(VColor::Z(0));
                        cur_node_idx.push(cur_idx);
                        swap.1 = Some(i);
                    }
                }
            }

            if cnot.0.is_some() && cnot.1.is_some() {
                g.add_edge(cnot.0.unwrap(), cnot.1.unwrap());
            }
            if swap.0.is_some() && swap.1.is_some() {
                let pos1 = swap.0.unwrap();
                let pos2 = swap.1.unwrap();

                let tmp = cur_node_idx[pos1];
                cur_node_idx[pos1] = cur_node_idx[pos2];
                cur_node_idx[pos2] = tmp;
            }
            if pre_nod_idx.len() != 0 {
                for (i, j) in pre_nod_idx.iter().enumerate() {
                    g.add_edge(*j, cur_node_idx[i]);
                }
            }
            pre_nod_idx = cur_node_idx.clone();
        }
        g
    }
}
