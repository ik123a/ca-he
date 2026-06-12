use std::ops::BitXor;
use rand::Rng;

/// Bit-parallel 1D binary CA grid.
/// Packs N cells into ceil(N/64) u64 words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitGrid {
    pub cells: Vec<u64>,
    pub size: usize,
}

impl BitGrid {
    /// Create a new BitGrid of a given size initialized to zero.
    pub fn new(size: usize) -> Self {
        let num_words = (size + 63) / 64;
        BitGrid {
            cells: vec![0u64; num_words],
            size,
        }
    }

    /// Create a BitGrid from a u64 value (for sizes <= 64).
    pub fn from_u64(val: u64, size: usize) -> Self {
        assert!(size <= 64);
        let mask = if size == 64 { u64::MAX } else { (1u64 << size) - 1 };
        BitGrid {
            cells: vec![val & mask],
            size,
        }
    }

    /// Convert a BitGrid to a u64 (for sizes <= 64).
    pub fn to_u64(&self) -> u64 {
        assert!(self.size <= 64);
        self.cells[0]
    }

    /// Create a BitGrid from a slice of bits.
    pub fn from_bits(bits: &[u8]) -> Self {
        let size = bits.len();
        let mut grid = Self::new(size);
        for (i, &bit) in bits.iter().enumerate() {
            if bit != 0 {
                grid.set_bit(i, true);
            }
        }
        grid
    }

    /// Convert a BitGrid to a vector of bits.
    pub fn to_bits(&self) -> Vec<u8> {
        let mut bits = vec![0u8; self.size];
        for i in 0..self.size {
            if self.get_bit(i) {
                bits[i] = 1;
            }
        }
        bits
    }

    /// Set the bit at a specific index.
    pub fn set_bit(&mut self, idx: usize, val: bool) {
        assert!(idx < self.size);
        let word = idx / 64;
        let bit = idx % 64;
        if val {
            self.cells[word] |= 1u64 << bit;
        } else {
            self.cells[word] &= !(1u64 << bit);
        }
    }

    /// Get the bit at a specific index.
    pub fn get_bit(&self, idx: usize) -> bool {
        assert!(idx < self.size);
        let word = idx / 64;
        let bit = idx % 64;
        ((self.cells[word] >> bit) & 1) != 0
    }

    /// Shift the grid left by 1 cell (periodic boundary conditions).
    /// bit[i] gets bit[i-1].
    pub fn shift_left(&self) -> Self {
        let K = self.cells.len();
        let mut result = vec![0u64; K];

        for w in 0..K {
            let prev_word = if w == 0 { K - 1 } else { w - 1 };
            let carry = if w == 0 {
                let last_active_bit_idx = (self.size - 1) % 64;
                (self.cells[prev_word] >> last_active_bit_idx) & 1
            } else {
                self.cells[prev_word] >> 63
            };
            result[w] = (self.cells[w] << 1) | carry;
        }

        // Apply mask to the last word
        let last_word_mask = if self.size % 64 == 0 {
            u64::MAX
        } else {
            (1u64 << (self.size % 64)) - 1
        };
        result[K - 1] &= last_word_mask;

        BitGrid {
            cells: result,
            size: self.size,
        }
    }

    /// Shift the grid right by 1 cell (periodic boundary conditions).
    /// bit[i] gets bit[i+1].
    pub fn shift_right(&self) -> Self {
        let K = self.cells.len();
        let mut result = vec![0u64; K];

        for w in 0..K {
            let next_word = if w == K - 1 { 0 } else { w + 1 };
            let carry = if w == K - 1 {
                self.cells[0] & 1
            } else {
                self.cells[next_word] & 1
            };
            
            if w == K - 1 {
                let last_active_bit_idx = (self.size - 1) % 64;
                result[w] = (self.cells[w] >> 1) | (carry << last_active_bit_idx);
            } else {
                result[w] = (self.cells[w] >> 1) | (carry << 63);
            }
        }

        // Apply mask to the last word
        let last_word_mask = if self.size % 64 == 0 {
            u64::MAX
        } else {
            (1u64 << (self.size % 64)) - 1
        };
        result[K - 1] &= last_word_mask;

        BitGrid {
            cells: result,
            size: self.size,
        }
    }
}

// BitXor trait implementation for BitGrid
impl<'a, 'b> BitXor<&'b BitGrid> for &'a BitGrid {
    type Output = BitGrid;

    fn bitxor(self, rhs: &'b BitGrid) -> BitGrid {
        assert_eq!(self.size, rhs.size);
        let cells = self.cells
            .iter()
            .zip(rhs.cells.iter())
            .map(|(a, b)| a ^ b)
            .collect();
        BitGrid {
            cells,
            size: self.size,
        }
    }
}

impl BitXor for BitGrid {
    type Output = BitGrid;

    fn bitxor(self, rhs: BitGrid) -> BitGrid {
        &self ^ &rhs
    }
}

/// Reversible CA simulator using second-order (Fredkin) construction.
pub struct ReversibleCA {
    pub rule_lut: u8,
    pub steps: usize,
}

impl ReversibleCA {
    pub fn new(rule_lut: u8, steps: usize) -> Self {
        ReversibleCA { rule_lut, steps }
    }

    /// Evolve the CA forward by `steps` steps.
    /// prev = s(t-1), curr = s(t)
    /// Returns (new_prev, new_curr) after `steps` applications
    pub fn evolve(&self, prev: &BitGrid, curr: &BitGrid) -> (BitGrid, BitGrid) {
        let mut p = prev.clone();
        let mut c = curr.clone();
        let mut temp = BitGrid::new(prev.size);
        for _ in 0..self.steps {
            self.apply_rule_inplace_step(&p, &c, &mut temp);
            std::mem::swap(&mut p, &mut c);
            std::mem::swap(&mut c, &mut temp);
        }
        (p, c)
    }

    /// Reverse-evolve the CA by `steps` steps.
    /// Returns (orig_prev, orig_curr)
    pub fn reverse(&self, prev: &BitGrid, curr: &BitGrid) -> (BitGrid, BitGrid) {
        // Second-order CA: reverse is swap, evolve forward, swap
        let (p, c) = self.evolve(curr, prev);
        (c, p) // Swapped back
    }

    /// Apply the Wolfram local rule in parallel to all cells in the grid.
    pub fn apply_rule(&self, grid: &BitGrid) -> BitGrid {
        let L = grid.shift_left();
        let C = grid;
        let R = grid.shift_right();

        let mut result = vec![0u64; grid.cells.len()];

        for w in 0..grid.cells.len() {
            let l = L.cells[w];
            let c = C.cells[w];
            let r = R.cells[w];

            let not_l = !l;
            let not_c = !c;
            let not_r = !r;

            let mut out = 0u64;
            let rule = self.rule_lut;

            if (rule & 1) != 0 { out |= not_l & not_c & not_r; }
            if (rule & 2) != 0 { out |= not_l & not_c & r; }
            if (rule & 4) != 0 { out |= not_l & c & not_r; }
            if (rule & 8) != 0 { out |= not_l & c & r; }
            if (rule & 16) != 0 { out |= l & not_c & not_r; }
            if (rule & 32) != 0 { out |= l & not_c & r; }
            if (rule & 64) != 0 { out |= l & c & not_r; }
            if (rule & 128) != 0 { out |= l & c & r; }

            result[w] = out;
        }

        // Apply mask to the last word
        let last_word_mask = if grid.size % 64 == 0 {
            u64::MAX
        } else {
            (1u64 << (grid.size % 64)) - 1
        };
        result[grid.cells.len() - 1] &= last_word_mask;

        BitGrid {
            cells: result,
            size: grid.size,
        }
    }

    /// In-place step of rule application.
    /// Computes `temp = apply_rule(c) ^ p` directly without allocation.
    fn apply_rule_inplace_step(&self, p: &BitGrid, c: &BitGrid, temp: &mut BitGrid) {
        let k_len = c.cells.len();
        let size = c.size;
        let rule = self.rule_lut;

        if k_len == 1 {
            let c_word = c.cells[0];
            let last_active_bit_idx = (size - 1) % 64;
            let carry_l = (c_word >> last_active_bit_idx) & 1;
            let l = (c_word << 1) | carry_l;
            
            let carry_r = c_word & 1;
            let r = (c_word >> 1) | (carry_r << last_active_bit_idx);
            
            let not_l = !l;
            let not_c = !c_word;
            let not_r = !r;

            let mut out = 0u64;
            if (rule & 1) != 0 { out |= not_l & not_c & not_r; }
            if (rule & 2) != 0 { out |= not_l & not_c & r; }
            if (rule & 4) != 0 { out |= not_l & c_word & not_r; }
            if (rule & 8) != 0 { out |= not_l & c_word & r; }
            if (rule & 16) != 0 { out |= l & not_c & not_r; }
            if (rule & 32) != 0 { out |= l & not_c & r; }
            if (rule & 64) != 0 { out |= l & c_word & not_r; }
            if (rule & 128) != 0 { out |= l & c_word & r; }

            let last_word_mask = if size % 64 == 0 {
                u64::MAX
            } else {
                (1u64 << (size % 64)) - 1
            };
            out &= last_word_mask;
            temp.cells[0] = out ^ p.cells[0];
            return;
        }

        // w = 0
        {
            let c_word = c.cells[0];
            let carry_l = {
                let last_active_bit_idx = (size - 1) % 64;
                (c.cells[k_len - 1] >> last_active_bit_idx) & 1
            };
            let l = (c_word << 1) | carry_l;

            let carry_r = c.cells[1] & 1;
            let r = (c_word >> 1) | (carry_r << 63);

            let not_l = !l;
            let not_c = !c_word;
            let not_r = !r;

            let mut out = 0u64;
            if (rule & 1) != 0 { out |= not_l & not_c & not_r; }
            if (rule & 2) != 0 { out |= not_l & not_c & r; }
            if (rule & 4) != 0 { out |= not_l & c_word & not_r; }
            if (rule & 8) != 0 { out |= not_l & c_word & r; }
            if (rule & 16) != 0 { out |= l & not_c & not_r; }
            if (rule & 32) != 0 { out |= l & not_c & r; }
            if (rule & 64) != 0 { out |= l & c_word & not_r; }
            if (rule & 128) != 0 { out |= l & c_word & r; }

            temp.cells[0] = out ^ p.cells[0];
        }

        // w = 1..k_len-1
        for w in 1..k_len - 1 {
            let c_word = c.cells[w];
            let carry_l = c.cells[w - 1] >> 63;
            let l = (c_word << 1) | carry_l;

            let carry_r = c.cells[w + 1] & 1;
            let r = (c_word >> 1) | (carry_r << 63);

            let not_l = !l;
            let not_c = !c_word;
            let not_r = !r;

            let mut out = 0u64;
            if (rule & 1) != 0 { out |= not_l & not_c & not_r; }
            if (rule & 2) != 0 { out |= not_l & not_c & r; }
            if (rule & 4) != 0 { out |= not_l & c_word & not_r; }
            if (rule & 8) != 0 { out |= not_l & c_word & r; }
            if (rule & 16) != 0 { out |= l & not_c & not_r; }
            if (rule & 32) != 0 { out |= l & not_c & r; }
            if (rule & 64) != 0 { out |= l & c_word & not_r; }
            if (rule & 128) != 0 { out |= l & c_word & r; }

            temp.cells[w] = out ^ p.cells[w];
        }

        // w = k_len - 1
        {
            let w = k_len - 1;
            let c_word = c.cells[w];
            let carry_l = c.cells[w - 1] >> 63;
            let l = (c_word << 1) | carry_l;

            let carry_r = c.cells[0] & 1;
            let last_active_bit_idx = (size - 1) % 64;
            let r = (c_word >> 1) | (carry_r << last_active_bit_idx);

            let not_l = !l;
            let not_c = !c_word;
            let not_r = !r;

            let mut out = 0u64;
            if (rule & 1) != 0 { out |= not_l & not_c & not_r; }
            if (rule & 2) != 0 { out |= not_l & not_c & r; }
            if (rule & 4) != 0 { out |= not_l & c_word & not_r; }
            if (rule & 8) != 0 { out |= not_l & c_word & r; }
            if (rule & 16) != 0 { out |= l & not_c & not_r; }
            if (rule & 32) != 0 { out |= l & not_c & r; }
            if (rule & 64) != 0 { out |= l & c_word & not_r; }
            if (rule & 128) != 0 { out |= l & c_word & r; }

            let last_word_mask = if size % 64 == 0 {
                u64::MAX
            } else {
                (1u64 << (size % 64)) - 1
            };
            out &= last_word_mask;

            temp.cells[w] = out ^ p.cells[w];
        }
    }
}

/// Encrypt a plaintext using the reversible CA.
pub fn encrypt(plaintext: &BitGrid, iv: &BitGrid, rule_lut: u8, steps: usize) -> (BitGrid, BitGrid) {
    let initial_prev = plaintext ^ iv;
    let initial_curr = iv.clone();
    let ca = ReversibleCA::new(rule_lut, steps);
    ca.evolve(&initial_prev, &initial_curr)
}

/// Decrypt a ciphertext pair back to plaintext.
pub fn decrypt(c0: &BitGrid, c1: &BitGrid, iv: &BitGrid, rule_lut: u8, steps: usize) -> BitGrid {
    let ca = ReversibleCA::new(rule_lut, steps);
    let (orig_prev, _orig_curr) = ca.reverse(c0, c1);
    &orig_prev ^ iv
}

/// Encode a k-bit plaintext (represented as u64) into an N-cell BitGrid using repetition coding.
/// Each bit is repeated N/k times.
pub fn encode_repetition(val: u64, k: usize, n: usize) -> BitGrid {
    assert!(k > 0);
    assert!(n >= k);
    let r = n / k;
    let mut grid = BitGrid::new(n);
    for bit_idx in 0..k {
        let bit_val = ((val >> bit_idx) & 1) != 0;
        for j in 0..r {
            let idx = bit_idx * r + j;
            if idx < n {
                grid.set_bit(idx, bit_val);
            }
        }
    }
    grid
}

/// Decode an N-cell BitGrid back to a k-bit plaintext u64 using majority voting.
pub fn decode_repetition(grid: &BitGrid, k: usize) -> u64 {
    let n = grid.size;
    assert!(k > 0);
    assert!(n >= k);
    let r = n / k;
    let mut val = 0u64;
    for bit_idx in 0..k {
        let start = bit_idx * r;
        let end = if bit_idx == k - 1 { n } else { (bit_idx + 1) * r };
        let mut ones = 0;
        let count = end - start;
        for i in start..end {
            if grid.get_bit(i) {
                ones += 1;
            }
        }
        if ones > count / 2 {
            val |= 1u64 << bit_idx;
        }
    }
    val
}

// ─────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shifts_single_word() {
        let size = 8;
        // 00001011 = 11
        let grid = BitGrid::from_u64(11, size);

        // Shift left: bit i gets bit i-1.
        // 00001011 shifted left becomes 00010110 = 22
        let l = grid.shift_left();
        assert_eq!(l.to_u64(), 22);

        // Shift right: bit i gets bit i+1.
        // 00001011 shifted right becomes 10000101 = 133
        // (bit 0 wrap-arounds to bit 7)
        let r = grid.shift_right();
        assert_eq!(r.to_u64(), 133);
    }

    #[test]
    fn test_shifts_multi_word() {
        let size = 128; // Exactly 2 words
        let mut grid = BitGrid::new(size);
        grid.set_bit(0, true);
        grid.set_bit(63, true);
        grid.set_bit(64, true);

        // Shift left:
        // Bit 0 moves to 1
        // Bit 63 moves to 64
        // Bit 64 moves to 65
        // Bit 127 (0) wrap-arounds to 0
        let l = grid.shift_left();
        assert!(l.get_bit(1));
        assert!(l.get_bit(64));
        assert!(l.get_bit(65));
        assert!(!l.get_bit(0));

        // Shift right:
        // Bit 0 moves to 127
        // Bit 63 moves to 62
        // Bit 64 moves to 63
        let r = grid.shift_right();
        assert!(r.get_bit(127));
        assert!(r.get_bit(62));
        assert!(r.get_bit(63));
    }

    #[test]
    fn test_roundtrip_encryption() {
        let mut rng = rand::thread_rng();
        let sizes = [8, 16, 64, 128, 256];
        
        for &size in &sizes {
            let ca_rule = rng.gen::<u8>();
            let steps = 32;

            let mut plaintext = BitGrid::new(size);
            let mut iv = BitGrid::new(size);
            
            // Fill with random bits
            for i in 0..size {
                plaintext.set_bit(i, rng.gen::<bool>());
                iv.set_bit(i, rng.gen::<bool>());
            }

            let (c0, c1) = encrypt(&plaintext, &iv, ca_rule, steps);
            let decrypted = decrypt(&c0, &c1, &iv, ca_rule, steps);

            assert_eq!(decrypted, plaintext, "Roundtrip failed for size {}", size);
        }
    }

    #[test]
    fn test_linear_homomorphism() {
        // Rule 90 is linear (additive): f(L, C, R) = L ^ R
        let size = 64;
        let rule_lut = 90;
        let steps = 16;
        let iv = BitGrid::new(size); // Zero IV

        let mut rng = rand::thread_rng();
        let mut a = BitGrid::new(size);
        let mut b = BitGrid::new(size);
        
        for i in 0..size {
            a.set_bit(i, rng.gen::<bool>());
            b.set_bit(i, rng.gen::<bool>());
        }

        let a_xor_b = &a ^ &b;

        let (c_a0, c_a1) = encrypt(&a, &iv, rule_lut, steps);
        let (c_b0, c_b1) = encrypt(&b, &iv, rule_lut, steps);

        let c_sum0 = &c_a0 ^ &c_b0;
        let c_sum1 = &c_a1 ^ &c_b1;

        let decrypted = decrypt(&c_sum0, &c_sum1, &iv, rule_lut, steps);

        assert_eq!(decrypted, a_xor_b, "XOR homomorphism failed for Rule 90");
    }

    #[test]
    fn test_repetition_coding() {
        let val = 0b10101100u64; // 8-bit value
        let k = 8;
        let n = 64;
        let encoded = encode_repetition(val, k, n);
        
        // Verify encoding repetition
        // bit 0 is 0 -> cells 0..8 should be 0
        // bit 1 is 0 -> cells 8..16 should be 0
        // bit 2 is 1 -> cells 16..24 should be 1
        for i in 0..8 { assert!(!encoded.get_bit(i)); }
        for i in 16..24 { assert!(encoded.get_bit(i)); }

        let decoded = decode_repetition(&encoded, k);
        assert_eq!(decoded, val);

        // Add some noise (e.g. flip 3 bits in each 8-bit block)
        let mut noisy = encoded.clone();
        for bit_idx in 0..k {
            noisy.set_bit(bit_idx * 8 + 0, !noisy.get_bit(bit_idx * 8 + 0));
            noisy.set_bit(bit_idx * 8 + 1, !noisy.get_bit(bit_idx * 8 + 1));
            noisy.set_bit(bit_idx * 8 + 2, !noisy.get_bit(bit_idx * 8 + 2));
        }
        
        // Majority vote should still correctly decode
        let decoded_noisy = decode_repetition(&noisy, k);
        assert_eq!(decoded_noisy, val, "Majority voting failed under noise");
    }
}
