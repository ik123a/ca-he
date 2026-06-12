# CA-HE: Quantum-Resilient Homomorphic Encryption via Cellular Automata Evolution

A novel cryptographic system that combines homomorphic encryption with reversible cellular automata (CA), using evolutionary strategies to discover rule pairs that enable homomorphic operations on encrypted data.

## Project Structure

```
ca-he/
  rust/                     # High-performance Rust CA engine
    src/
      lib.rs                # 1D/2D BitGrid and ReversibleCA implementations
      bin/
        search.rs           # Rayon-parallelized 1D rule search
        search2d.rs         # Rayon-parallelized 2D rule search
    benches/                # Criterion performance benchmarks
  blockchain/               # Proof-of-Evolution Blockchain integration
    contracts/
      CAHERuleRegistry.sol  # Solidity contract with row-wise 2D CA simulation
    test/
      test_registry.js      # Hardhat unit tests
    scripts/                # Miner and automation scripts
    run_miner.ps1           # Coordinate challenge retrieval, search, and submission
  src/                      # Original Python prototype CA engine
    __init__.py
    ca_core.py              # Core CA simulation engine
  exhaustive_search.py      # Day-1 validation: brute-force all rule pairs
  evolutionary_search.py    # Python prototype NSGA-II rule pair discovery
  results/                  # Search results (JSON)
  benchmarks/               # Performance benchmarks
  tests/                    # Test suite
```

## Quick Start

### Rust Engine & Search
```bash
cd rust
# Run unit tests
powershell -File run_tests.ps1

# Run 1D Rayon-parallelized search
powershell -File run_search.ps1

# Run 2D Rayon-parallelized search
powershell -File run_search2d.ps1
```

### Blockchain & Miner
```bash
cd blockchain
# Compile contracts and run tests
npx hardhat test

# Run Proof-of-Evolution miner pipeline (starts local node, runs miner, claims reward)
powershell -File run_miner.ps1
```

## Core Concepts

### Second-Order Reversible CA (Fredkin Construction)
- **Forward:** `s(t+1) = f(neighborhood(s(t))) XOR s(t-1)`
- **Backward:** Swap prev/curr, evolve forward, swap back
- Guaranteed reversible for ANY local rule f

### Encryption Scheme
- **Key:** `(rule_lut, steps, IV)`
- **Encrypt(m):** Evolve `(m XOR IV, IV)` forward for `steps` steps
- **Decrypt(c):** Reverse-evolve the ciphertext, XOR with IV

### Homomorphic Property
- **Goal:** Find rules where operating on ciphertexts corresponds to operating on plaintexts
- **XOR homomorphism:** `Decrypt(Enc(a) XOR Enc(b)) = a XOR b`
- **Addition/XOR 2D homomorphism:** Evolving XOR-combined ciphertexts under an evaluation rule

## Status

- [x] Core CA engine (1D, binary, radius-1)
- [x] Encrypt/decrypt with roundtrip verification
- [x] Exhaustive search over all 65K rule pairs
- [x] NSGA-II evolutionary search (Python prototype)
- [x] High-performance Rust Core engine (1D/2D BitGrid with Rayon concurrency)
- [x] Gas-optimized Solidity contract for on-chain 2D CA verification
- [x] Proof-of-Evolution Blockchain Miner node & pipeline
- [ ] libcahe C-API & Python ctypes bindings (Phase 5)
- [ ] Performance benchmarks vs TFHE-rs (Phase 5)
- [ ] Cryptographic security analysis & NIST tests (Phase 5)

## License

Apache 2.0 + MIT dual license

