import unittest
import sys
import os

# Add src to python path to import bindings
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "src")))

import cahe_bindings

class TestCaheBindings(unittest.TestCase):
    def test_repetition_coding(self):
        val = 0b10101100
        k = 8
        n = 64
        encoded = cahe_bindings.encode_repetition_1d(val, k, n)
        # Verify it encodes correctly
        self.assertNotEqual(encoded, 0)
        
        decoded = cahe_bindings.decode_repetition_1d(encoded, k, n)
        self.assertEqual(decoded, val)

    def test_1d_roundtrip(self):
        # Rule 30, steps 16, grid size 64, arbitrary IV and plaintext
        enc_rule = 30
        eval_rule = 90
        steps = 16
        size = 64
        iv = 0x123456789abcdef0
        pt = 0xabcdef0123456789

        key = cahe_bindings.keygen_1d(enc_rule, eval_rule, steps, iv)
        ct = cahe_bindings.encrypt_1d(key, pt, size)
        
        self.assertNotEqual(ct.c0, 0)
        self.assertNotEqual(ct.c1, 0)

        recovered = cahe_bindings.decrypt_1d(key, ct, size)
        self.assertEqual(recovered, pt)

    def test_1d_homomorphism_rule90(self):
        # Rule 90 is linear (additive): f(L, C, R) = L ^ R
        # This gives exact XOR homomorphism with eval_rule = 90
        enc_rule = 90
        eval_rule = 90
        steps = 8
        size = 64
        iv = 0  # Zero IV for simplicity in linear check
        a = 0x0f0f0f0f0f0f0f0f
        b = 0xf0f0f0f0f0f0f0f0

        key = cahe_bindings.keygen_1d(enc_rule, eval_rule, steps, iv)
        
        ct_a = cahe_bindings.encrypt_1d(key, a, size)
        ct_b = cahe_bindings.encrypt_1d(key, b, size)
        
        ct_sum = cahe_bindings.eval_add_1d(key, ct_a, ct_b, size)
        
        recovered_sum = cahe_bindings.decrypt_1d(key, ct_sum, size)
        self.assertEqual(recovered_sum, a ^ b)

    def test_1d_homomorphism_discovered_rule(self):
        # Discovered non-linear homomorphic rule pair: enc=43, eval=36
        # From our search results, this has homo_xor = 1.0 (with 8-bit plaintext repetition-coded into 64-bit grid)
        enc_rule = 43
        eval_rule = 36
        steps = 44
        size = 64
        iv = 0
        
        k = 8
        val_a = 0b10110011
        val_b = 0b01011100
        
        pt_a = cahe_bindings.encode_repetition_1d(val_a, k, size)
        pt_b = cahe_bindings.encode_repetition_1d(val_b, k, size)
        
        key = cahe_bindings.keygen_1d(enc_rule, eval_rule, steps, iv)
        
        ct_a = cahe_bindings.encrypt_1d(key, pt_a, size)
        ct_b = cahe_bindings.encrypt_1d(key, pt_b, size)
        
        ct_sum = cahe_bindings.eval_add_1d(key, ct_a, ct_b, size)
        
        recovered_sum_grid = cahe_bindings.decrypt_1d(key, ct_sum, size)
        recovered_sum_val = cahe_bindings.decode_repetition_1d(recovered_sum_grid, k, size)
        
        self.assertEqual(recovered_sum_val, val_a ^ val_b)

    def test_2d_roundtrip(self):
        # Von Neumann rule (32-bit), height=8, width=8 (64 cells)
        enc_rule = 0x20202020  # Rule 64 equivalent
        eval_rule = 0x20202020
        steps = 16
        height = 8
        width = 8
        iv = 0x5555555555555555
        pt = 0xaaaaaaaaaaaaaaaa

        key = cahe_bindings.keygen_2d(enc_rule, eval_rule, steps, iv)
        ct = cahe_bindings.encrypt_2d(key, pt, height, width)
        
        self.assertNotEqual(ct.c0, 0)
        self.assertNotEqual(ct.c1, 0)

        recovered = cahe_bindings.decrypt_2d(key, ct, height, width)
        self.assertEqual(recovered, pt)

    def test_2d_homomorphism_linear(self):
        # Construct a 2D linear rule (XOR of neighbors)
        linear_rule = 0
        for idx in range(32):
            bit = ((idx >> 4) & 1) ^ ((idx >> 3) & 1) ^ ((idx >> 2) & 1) ^ ((idx >> 1) & 1) ^ (idx & 1)
            linear_rule |= (bit << idx)
            
        steps = 8
        height = 8
        width = 8
        iv = 0
        a = 0x0f0f0f0f0f0f0f0f
        b = 0xf0f0f0f0f0f0f0f0

        key = cahe_bindings.keygen_2d(linear_rule, linear_rule, steps, iv)
        
        ct_a = cahe_bindings.encrypt_2d(key, a, height, width)
        ct_b = cahe_bindings.encrypt_2d(key, b, height, width)
        
        ct_sum = cahe_bindings.eval_add_2d(key, ct_a, ct_b, height, width)
        
        recovered_sum = cahe_bindings.decrypt_2d(key, ct_sum, height, width)
        self.assertEqual(recovered_sum, a ^ b)

    def test_3d_roundtrip(self):
        rule_lut0 = 0x123456789abcdef0
        rule_lut1 = 0xfedcba9876543210
        steps = 8
        depth = 4
        height = 4
        width = 4
        iv = 0x5555555555555555
        pt = 0xaaaaaaaaaaaaaaaa

        key = cahe_bindings.keygen_3d(rule_lut0, rule_lut1, steps, iv)
        ct = cahe_bindings.encrypt_3d(key, pt, depth, height, width)
        
        self.assertNotEqual(ct.c0, 0)
        self.assertNotEqual(ct.c1, 0)

        recovered = cahe_bindings.decrypt_3d(key, ct, depth, height, width)
        self.assertEqual(recovered, pt)

    def test_3d_homomorphism_linear(self):
        # Generate 3D linear rule: XOR of all 7 neighbors
        rule_lut0 = 0
        rule_lut1 = 0
        for idx in range(128):
            bit_f = (idx >> 6) & 1
            bit_b = (idx >> 5) & 1
            bit_u = (idx >> 4) & 1
            bit_d = (idx >> 3) & 1
            bit_l = (idx >> 2) & 1
            bit_r = (idx >> 1) & 1
            bit_c = idx & 1
            
            bit_out = bit_f ^ bit_b ^ bit_u ^ bit_d ^ bit_l ^ bit_r ^ bit_c
            if idx < 64:
                rule_lut0 |= (bit_out << idx)
            else:
                rule_lut1 |= (bit_out << (idx - 64))

        steps = 8
        depth = 4
        height = 4
        width = 4
        iv = 0
        a = 0x0f0f0f0f0f0f0f0f
        b = 0xf0f0f0f0f0f0f0f0

        key = cahe_bindings.keygen_3d(rule_lut0, rule_lut1, steps, iv)
        
        ct_a = cahe_bindings.encrypt_3d(key, a, depth, height, width)
        ct_b = cahe_bindings.encrypt_3d(key, b, depth, height, width)
        
        ct_sum = cahe_bindings.eval_add_3d(key, ct_a, ct_b, depth, height, width)
        
        recovered_sum = cahe_bindings.decrypt_3d(key, ct_sum, depth, height, width)
        self.assertEqual(recovered_sum, a ^ b)

if __name__ == "__main__":
    unittest.main()

