"""
CA-HE Core: 1D Binary Cellular Automata Engine with Reversible (Fredkin) Construction

This module provides the fundamental CA simulation primitives for the CA-HE
cryptographic system. All states are packed into Python integers for speed.

Grid: N cells, each binary (0 or 1), with periodic boundary conditions.
Rule: Wolfram-style 8-bit LUT for radius-1 (3-cell neighborhood).
Reversibility: Second-order (Fredkin) construction:
    s(t+1) = f(neighborhood(s(t))) XOR s(t-1)
"""

import random
import time


def apply_rule_1d(state: int, rule_lut: int, n: int) -> int:
    """
    Apply a 1D binary CA rule (radius-1) to all cells simultaneously.
    
    Args:
        state: N-bit integer representing the current CA grid
        rule_lut: 8-bit Wolfram number encoding the rule LUT
        n: Grid size (number of cells/bits)
    
    Returns:
        New state as an N-bit integer
    
    The neighborhood for cell i is (left, center, right) = (cell[i-1], cell[i], cell[i+1])
    with periodic boundary conditions. The 3-bit index into the LUT is:
        index = (left << 2) | (center << 1) | right
    """
    mask = (1 << n) - 1
    new_state = 0
    
    for i in range(n):
        # Extract neighborhood bits with periodic boundaries
        left = (state >> ((i - 1) % n)) & 1
        center = (state >> i) & 1
        right = (state >> ((i + 1) % n)) & 1
        
        # 3-bit neighborhood index
        neighborhood = (left << 2) | (center << 1) | right
        
        # Look up the rule output
        output_bit = (rule_lut >> neighborhood) & 1
        
        # Set the output bit
        new_state |= (output_bit << i)
    
    return new_state & mask


def evolve_reversible(prev: int, curr: int, rule_lut: int, n: int, steps: int) -> tuple:
    """
    Evolve a second-order reversible CA forward for `steps` time steps.
    
    The Fredkin construction: s(t+1) = f(s(t)) XOR s(t-1)
    This is guaranteed to be reversible for ANY local rule f.
    
    Args:
        prev: Previous state s(t-1) as N-bit integer
        curr: Current state s(t) as N-bit integer
        rule_lut: 8-bit Wolfram number
        n: Grid size
        steps: Number of evolution steps
    
    Returns:
        (new_prev, new_curr) after `steps` applications
    """
    mask = (1 << n) - 1
    p, c = prev & mask, curr & mask
    
    for _ in range(steps):
        next_state = apply_rule_1d(c, rule_lut, n) ^ p
        p = c
        c = next_state & mask
    
    return (p, c)


def reverse_evolve(prev: int, curr: int, rule_lut: int, n: int, steps: int) -> tuple:
    """
    Reverse-evolve a second-order reversible CA for `steps` time steps.
    
    For second-order CA, reversing is: swap (prev, curr), evolve forward, swap back.
    This works because if (p, c) -> (c, f(c)^p), then
    starting from (c, p) and evolving gives (p, f(p)^c),
    which after swap gives the original predecessor.
    
    Args:
        prev: The "prev" component of the state to reverse from
        curr: The "curr" component of the state to reverse from
        rule_lut: Same rule used for forward evolution
        n: Grid size
        steps: Number of steps to reverse
    
    Returns:
        (orig_prev, orig_curr) such that evolve_reversible(orig_prev, orig_curr, ..., steps) == (prev, curr)
    """
    # Reverse = swap, evolve forward, swap
    p, c = evolve_reversible(curr, prev, rule_lut, n, steps)
    return (c, p)


def encrypt(plaintext: int, iv: int, rule_lut: int, n: int, steps: int) -> tuple:
    """
    Encrypt a plaintext using the reversible CA.
    
    Encryption: Evolve (plaintext XOR iv, iv) forward for `steps` steps.
    
    Args:
        plaintext: N-bit plaintext integer
        iv: N-bit initialization vector (secret nonce)
        rule_lut: 8-bit encryption rule (secret key component)
        n: Grid size
        steps: Number of evolution steps (secret key component)
    
    Returns:
        Ciphertext as (c0, c1) pair of N-bit integers
    """
    mask = (1 << n) - 1
    initial_prev = (plaintext ^ iv) & mask
    initial_curr = iv & mask
    return evolve_reversible(initial_prev, initial_curr, rule_lut, n, steps)


def decrypt(c0: int, c1: int, iv: int, rule_lut: int, n: int, steps: int) -> int:
    """
    Decrypt a ciphertext pair back to plaintext.
    
    Decryption: Reverse-evolve (c0, c1) for `steps` steps, then XOR with iv.
    
    Args:
        c0, c1: Ciphertext pair
        iv: Same IV used during encryption
        rule_lut: Same rule used during encryption
        n: Grid size
        steps: Same number of steps used during encryption
    
    Returns:
        Recovered plaintext as N-bit integer
    """
    mask = (1 << n) - 1
    orig_prev, orig_curr = reverse_evolve(c0, c1, rule_lut, n, steps)
    # orig_prev = plaintext ^ iv, orig_curr = iv
    return (orig_prev ^ iv) & mask


def test_roundtrip(n: int = 8, steps: int = 32, num_tests: int = 100):
    """
    Verify that encrypt-then-decrypt correctly recovers the plaintext.
    Tests with random plaintexts, IVs, and rules.
    """
    mask = (1 << n) - 1
    random.seed(12345)
    
    passed = 0
    failed = 0
    
    print(f"Testing roundtrip: N={n}, steps={steps}")
    
    for i in range(num_tests):
        plaintext = random.randint(0, mask)
        iv = random.randint(0, mask)
        rule_lut = random.randint(0, 255)
        
        # Encrypt
        c0, c1 = encrypt(plaintext, iv, rule_lut, n, steps)
        
        # Decrypt
        recovered = decrypt(c0, c1, iv, rule_lut, n, steps)
        
        if recovered == plaintext:
            passed += 1
        else:
            failed += 1
            if failed <= 5:  # Only print first 5 failures
                print(f"  FAIL: plaintext={plaintext:0{n}b}, recovered={recovered:0{n}b}, "
                      f"rule={rule_lut}, iv={iv:0{n}b}")
    
    print(f"Passed {passed}/{num_tests} roundtrip tests!")
    
    if failed == 0:
        print("All roundtrip tests passed!")
    else:
        print(f"WARNING: {failed} tests failed!")
    
    return failed == 0


def benchmark(n: int = 8, steps: int = 32, iterations: int = 10000):
    """Benchmark CA evolution speed."""
    mask = (1 << n) - 1
    random.seed(42)
    
    prev = random.randint(0, mask)
    curr = random.randint(0, mask)
    rule_lut = 30  # Wolfram Rule 30 (known chaotic)
    
    start = time.perf_counter()
    for _ in range(iterations):
        prev, curr = evolve_reversible(prev, curr, rule_lut, n, steps)
    elapsed = time.perf_counter() - start
    
    total_cell_updates = iterations * steps * n
    cell_updates_per_sec = total_cell_updates / elapsed
    
    print(f"\nBenchmark: N={n}, steps={steps}, iterations={iterations}")
    print(f"  Time: {elapsed:.3f}s")
    print(f"  Cell updates/sec: {cell_updates_per_sec:,.0f}")
    print(f"  Encrypt+decrypt ops/sec: {iterations / elapsed:,.0f}")


if __name__ == '__main__':
    # Run tests
    success = test_roundtrip(n=8, steps=32)
    
    if success:
        # Also test with different grid sizes
        test_roundtrip(n=4, steps=16, num_tests=50)
        test_roundtrip(n=16, steps=32, num_tests=50)
        
        # Benchmark
        benchmark(n=8, steps=32)
