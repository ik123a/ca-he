"""
CA-HE Exhaustive Search: The Critical Day-1 Experiment

Tests ALL 65,536 possible rule pairs (enc_rule x eval_rule) for 1D binary CA
with radius-1 and grid size N=8, looking for homomorphic properties.

Two modes:
  Mode 1 (Direct XOR): Encrypt(a) XOR Encrypt(b) decrypts to a XOR b?
  Mode 2 (Eval-Rule): Apply eval_rule to combined ciphertext, then decrypt.
    - Sub-mode A: Check for XOR homomorphism (a ^ b)
    - Sub-mode B: Check for modular addition homomorphism ((a + b) % 256)

This search should complete in under 5 minutes on modern hardware.
"""

import sys
import os
import time
import json
import random

# Add src to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'src'))
from ca_core import encrypt, decrypt, evolve_reversible, apply_rule_1d


def exhaustive_search(n=8, steps=16, num_test_pairs=50):
    """
    Exhaustively search all 256 x 256 = 65,536 rule pairs.
    
    For speed, test on a random sample of plaintext pairs rather than
    all 256*256 = 65,536 possible pairs.
    """
    mask = (1 << n) - 1
    iv = 0  # Fixed IV for consistency

    # Generate deterministic random test pairs
    random.seed(42)
    test_pairs = [(random.randint(0, mask), random.randint(0, mask)) 
                  for _ in range(num_test_pairs)]

    # Results storage
    mode1_results = []       # (enc_rule, xor_accuracy)
    mode2_xor_results = []   # (enc_rule, eval_rule, xor_accuracy)
    mode2_add_results = []   # (enc_rule, eval_rule, add_accuracy)

    print("=" * 70)
    print("CA-HE EXHAUSTIVE SEARCH")
    print("=" * 70)
    print(f"Grid size N={n}, evolution steps={steps}, test pairs={num_test_pairs}")
    print(f"Rule pair search space: 256 x 256 = 65,536")
    print(f"IV = {iv}")
    print()

    start = time.time()

    # ==================================================================
    # MODE 1: Direct XOR Homomorphism
    # ==================================================================
    # If the CA encryption is a linear map, then:
    #   Enc(a) XOR Enc(b) = Enc(a XOR b)
    # So decrypting (c_a XOR c_b) should give (a XOR b).
    # Only 256 rules to test (no eval_rule needed).

    print("-" * 70)
    print("MODE 1: Direct XOR Homomorphism (no eval rule)")
    print("  Testing: Decrypt(Enc(a) XOR Enc(b)) == a XOR b ?")
    print("-" * 70)

    for enc_rule in range(256):
        correct = 0
        for a, b in test_pairs:
            c_a = encrypt(a, iv, enc_rule, n, steps)
            c_b = encrypt(b, iv, enc_rule, n, steps)
            c_sum = (c_a[0] ^ c_b[0], c_a[1] ^ c_b[1])
            result = decrypt(c_sum[0], c_sum[1], iv, enc_rule, n, steps)
            if result == (a ^ b):
                correct += 1
        accuracy = correct / num_test_pairs
        if accuracy > 0.3:  # Store anything interesting
            mode1_results.append((enc_rule, accuracy))
            if accuracy >= 0.9:
                print(f"  * Rule {enc_rule:3d} (0x{enc_rule:02X}): accuracy = {accuracy:.3f}  <- HIGH")
            elif accuracy >= 0.7:
                print(f"  - Rule {enc_rule:3d} (0x{enc_rule:02X}): accuracy = {accuracy:.3f}")

    mode1_results.sort(key=lambda x: -x[1])
    m1_time = time.time() - start
    print(f"\n  Mode 1 complete in {m1_time:.2f}s")
    print(f"  Rules with accuracy > 30%: {len(mode1_results)}")
    if mode1_results:
        print(f"  Best rule: {mode1_results[0][0]} with accuracy {mode1_results[0][1]:.4f}")
    print()

    # ==================================================================
    # MODE 2: Eval-Rule Homomorphism
    # ==================================================================
    # Encrypt a and b -> combine via XOR -> apply eval_rule -> decrypt
    # Check both XOR and modular addition targets.

    print("-" * 70)
    print("MODE 2: Eval-Rule Homomorphism (256 x 256 search)")
    print("  Testing: Decrypt(R_eval(Enc(a) XOR Enc(b))) == a XOR b ?")
    print("           Decrypt(R_eval(Enc(a) XOR Enc(b))) == (a + b) mod 256 ?")
    print("-" * 70)

    best_xor = (0, 0, 0.0)
    best_add = (0, 0, 0.0)
    checked = 0

    for enc_rule in range(256):
        # Pre-compute encryptions for this enc_rule
        encrypted = {}
        for a, b in test_pairs:
            if a not in encrypted:
                encrypted[a] = encrypt(a, iv, enc_rule, n, steps)
            if b not in encrypted:
                encrypted[b] = encrypt(b, iv, enc_rule, n, steps)

        for eval_rule in range(256):
            xor_correct = 0
            add_correct = 0

            for a, b in test_pairs:
                c_a = encrypted[a]
                c_b = encrypted[b]
                combined = (c_a[0] ^ c_b[0], c_a[1] ^ c_b[1])
                
                # Apply eval rule to the combined ciphertext
                c_eval = evolve_reversible(combined[0], combined[1], eval_rule, n, steps)
                
                # Decrypt using the enc rule
                result = decrypt(c_eval[0], c_eval[1], iv, enc_rule, n, steps)

                if result == (a ^ b):
                    xor_correct += 1
                if result == ((a + b) & mask):
                    add_correct += 1

            xor_acc = xor_correct / num_test_pairs
            add_acc = add_correct / num_test_pairs

            if xor_acc > best_xor[2]:
                best_xor = (enc_rule, eval_rule, xor_acc)
            if add_acc > best_add[2]:
                best_add = (enc_rule, eval_rule, add_acc)

            if xor_acc > 0.7:
                mode2_xor_results.append((enc_rule, eval_rule, xor_acc))
                if xor_acc >= 0.9:
                    print(f"  * XOR enc={enc_rule:3d} eval={eval_rule:3d}: {xor_acc:.3f}")
            if add_acc > 0.7:
                mode2_add_results.append((enc_rule, eval_rule, add_acc))
                if add_acc >= 0.9:
                    print(f"  * ADD enc={enc_rule:3d} eval={eval_rule:3d}: {add_acc:.3f}")

            checked += 1

        if (enc_rule + 1) % 32 == 0:
            elapsed = time.time() - start
            eta = (elapsed / (enc_rule + 1)) * (256 - enc_rule - 1)
            print(f"  Progress: {enc_rule+1}/256 enc_rules "
                  f"({checked:,}/65,536 pairs) "
                  f"| {elapsed:.1f}s elapsed, ~{eta:.0f}s remaining")

    elapsed = time.time() - start

    # ==================================================================
    # RESULTS SUMMARY
    # ==================================================================
    print()
    print("=" * 70)
    print("RESULTS SUMMARY")
    print("=" * 70)
    print(f"Total time: {elapsed:.2f}s")
    
    print(f"\n{'-'*70}")
    print("MODE 1: Direct XOR Homomorphism")
    print(f"  Rules with accuracy > 30%: {len(mode1_results)}")
    perfect_m1 = [r for r in mode1_results if r[1] >= 0.99]
    high_m1 = [r for r in mode1_results if 0.9 <= r[1] < 0.99]
    print(f"  Rules with accuracy >= 99%: {len(perfect_m1)}")
    print(f"  Rules with accuracy >= 90%: {len(perfect_m1) + len(high_m1)}")
    if mode1_results:
        print(f"  Top 10:")
        for rule, acc in mode1_results[:10]:
            linear_tag = " [LINEAR]" if is_linear_rule(rule, n) else " [NONLINEAR]"
            print(f"    Rule {rule:3d} (0b{rule:08b}): {acc:.4f}{linear_tag}")

    print(f"\n{'─'*70}")
    print("MODE 2: Eval-Rule XOR Homomorphism")
    mode2_xor_results.sort(key=lambda x: -x[2])
    print(f"  Pairs with accuracy > 70%: {len(mode2_xor_results)}")
    print(f"  Best: enc={best_xor[0]}, eval={best_xor[1]}, accuracy={best_xor[2]:.4f}")
    if mode2_xor_results:
        print(f"  Top 10:")
        for enc, eva, acc in mode2_xor_results[:10]:
            enc_lin = "L" if is_linear_rule(enc, n) else "N"
            eva_lin = "L" if is_linear_rule(eva, n) else "N"
            print(f"    enc={enc:3d}[{enc_lin}] eval={eva:3d}[{eva_lin}]: {acc:.4f}")

    print(f"\n{'─'*70}")
    print("MODE 2: Eval-Rule ADDITION Homomorphism")
    mode2_add_results.sort(key=lambda x: -x[2])
    print(f"  Pairs with accuracy > 70%: {len(mode2_add_results)}")
    print(f"  Best: enc={best_add[0]}, eval={best_add[1]}, accuracy={best_add[2]:.4f}")
    if mode2_add_results:
        print(f"  Top 10:")
        for enc, eva, acc in mode2_add_results[:10]:
            enc_lin = "L" if is_linear_rule(enc, n) else "N"
            eva_lin = "L" if is_linear_rule(eva, n) else "N"
            print(f"    enc={enc:3d}[{enc_lin}] eval={eva:3d}[{eva_lin}]: {acc:.4f}")

    # ==================================================================
    # VERDICT
    # ==================================================================
    print(f"\n{'='*70}")
    print("VERDICT")
    print(f"{'='*70}")
    
    if any(acc >= 0.99 for _, acc in mode1_results):
        nonlinear_perfect = [(r, a) for r, a in mode1_results 
                            if a >= 0.99 and not is_linear_rule(r, n)]
        if nonlinear_perfect:
            print("[OK] NONLINEAR rules with perfect XOR homomorphism FOUND!")
            print("   -> The core hypothesis is VALIDATED.")
            print("   -> These rules provide both security (nonlinearity)")
            print("     and homomorphism simultaneously.")
        else:
            print("[!!] Only LINEAR rules achieve perfect XOR homomorphism.")
            print("   -> Expected (linear CA are trivially homomorphic).")
            print("   -> Need eval-rule approach for nonlinear encryption.")
    
    has_nonlinear_xor = any(
        acc >= 0.9 and not is_linear_rule(enc, n) 
        for enc, _, acc in mode2_xor_results
    )
    has_nonlinear_add = any(
        acc >= 0.9 and not is_linear_rule(enc, n) 
        for enc, _, acc in mode2_add_results
    )
    
    if has_nonlinear_xor:
        print("[OK] NONLINEAR enc + eval rule pairs with XOR homomorphism FOUND!")
    if has_nonlinear_add:
        print("[OK] NONLINEAR enc + eval rule pairs with ADD homomorphism FOUND!")
    
    if not has_nonlinear_xor and not has_nonlinear_add:
        best_nl_xor = max(
            [(e, v, a) for e, v, a in mode2_xor_results 
             if not is_linear_rule(e, n)],
            key=lambda x: x[2], default=(0, 0, 0.0)
        )
        best_nl_add = max(
            [(e, v, a) for e, v, a in mode2_add_results 
             if not is_linear_rule(e, n)],
            key=lambda x: x[2], default=(0, 0, 0.0)
        )
        print(f"[!!] No nonlinear pair reaches 90% accuracy.")
        print(f"   Best nonlinear XOR: {best_nl_xor[2]:.4f} (enc={best_nl_xor[0]}, eval={best_nl_xor[1]})")
        print(f"   Best nonlinear ADD: {best_nl_add[2]:.4f} (enc={best_nl_add[0]}, eval={best_nl_add[1]})")
        print(f"   -> Consider: larger grid, more steps, approximate homomorphism,")
        print(f"     or hybrid linear-nonlinear construction.")

    # ==================================================================
    # SAVE RESULTS
    # ==================================================================
    results = {
        'params': {
            'n': n, 'steps': steps, 
            'num_test_pairs': num_test_pairs, 'iv': iv
        },
        'time_seconds': elapsed,
        'mode1_top': mode1_results[:30],
        'mode2_xor_top': [(e, v, a) for e, v, a in mode2_xor_results[:30]],
        'mode2_add_top': [(e, v, a) for e, v, a in mode2_add_results[:30]],
        'mode2_xor_count': len(mode2_xor_results),
        'mode2_add_count': len(mode2_add_results),
        'best_xor_overall': list(best_xor),
        'best_add_overall': list(best_add),
    }

    results_path = os.path.join(os.path.dirname(__file__), 'results', 
                                'exhaustive_search_results.json')
    os.makedirs(os.path.dirname(results_path), exist_ok=True)
    with open(results_path, 'w') as f:
        json.dump(results, f, indent=2)
    print(f"\nResults saved to {results_path}")


def is_linear_rule(rule_lut: int, n: int = 8) -> bool:
    """
    Check if a CA rule is linear (affine) over GF(2).
    
    A rule f(a, b, c) is linear if it can be written as:
        f(a, b, c) = a_3*a XOR a_2*b XOR a_1*c XOR a_0
    where a_3, a_2, a_1, a_0 in {0, 1}.
    
    There are exactly 16 affine Boolean functions of 3 variables.
    """
    # An affine function of 3 variables has the form:
    # f(x2, x1, x0) = a3*x2 XOR a2*x1 XOR a1*x0 XOR a0
    # Test all 16 possibilities
    for a0 in range(2):
        for a1 in range(2):
            for a2 in range(2):
                for a3 in range(2):
                    match = True
                    for idx in range(8):
                        x0 = (idx >> 0) & 1
                        x1 = (idx >> 1) & 1
                        x2 = (idx >> 2) & 1
                        expected = (a3 * x2) ^ (a2 * x1) ^ (a1 * x0) ^ a0
                        actual = (rule_lut >> idx) & 1
                        if expected != actual:
                            match = False
                            break
                    if match:
                        return True
    return False


if __name__ == '__main__':
    # Run with smaller step count for faster search
    # (16 steps is enough to test homomorphism, 32+ for security)
    exhaustive_search(n=8, steps=16, num_test_pairs=50)
