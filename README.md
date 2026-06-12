# CA-HE: Quantum-Resilient Homomorphic Encryption via Cellular Automata Evolution

A novel cryptographic system that combines fully homomorphic encryption (FHE) with reversible cellular automata (CA), using evolutionary strategies to discover rule pairs that enable homomorphic operations on encrypted data.

## Project Structure

```
ca-he/
  src/
    __init__.py
    ca_core.py              # Core CA simulation engine
  exhaustive_search.py      # Day-1 validation: brute-force all rule pairs
  evolutionary_search.py    # NSGA-II multi-objective rule pair discovery
  results/                  # Search results (JSON)
  benchmarks/               # Performance benchmarks
  tests/                    # Test suite
```

## Quick Start

```bash
# 1. Verify the CA engine works
python src/ca_core.py

# 2. Run the exhaustive search (critical Day-1 experiment)
python exhaustive_search.py

# 3. Run evolutionary search for better rule pairs
python evolutionary_search.py
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
- **Addition homomorphism:** Using an eval rule to transform combined ciphertexts

## Status

- [x] Core CA engine (1D, binary, radius-1)
- [x] Encrypt/decrypt with roundtrip verification
- [x] Exhaustive search over all 65K rule pairs
- [x] NSGA-II evolutionary search
- [ ] 2D CA extension
- [ ] GPU acceleration (CUDA)
- [ ] Blockchain rule registry
- [ ] Benchmark vs TFHE

## License

Apache 2.0 + MIT dual license
