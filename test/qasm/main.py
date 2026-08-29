"""Validate cartographer's QASM2 export against Qiskit."""

import cmath

import numpy as np
from qiskit import QuantumCircuit
from qiskit.quantum_info import Operator

# --- 1. Fixed circuit: parses and matches a natively-built reference -------
qc = QuantumCircuit.from_qasm_file("./qcircuit.qasm")

# add swap gate to the first and the second
qc.swap(0, 1)
qc.cx(0, 1)
print(qc.draw(fold=200))

U_qiskit = Operator(qc)
print(U_qiskit.data)
