use crate::{
    encrypt, decrypt, BitGrid, ReversibleCA,
    encrypt_2d, decrypt_2d, BitGrid2D, CARule2D, ReversibleCA2D,
    encrypt_3d, decrypt_3d, BitGrid3D, CARule3D, ReversibleCA3D,
    encode_repetition, decode_repetition
};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CaheKey1D {
    pub enc_rule: u8,
    pub eval_rule: u8,
    pub steps: u32,
    pub iv: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CaheCiphertext1D {
    pub c0: u64,
    pub c1: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CaheKey2D {
    pub enc_rule: u32,
    pub eval_rule: u32,
    pub steps: u32,
    pub iv: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CaheCiphertext2D {
    pub c0: u64,
    pub c1: u64,
}

// Helper functions for 2D packing
fn u64_to_grid2d(val: u64, height: usize, width: usize) -> BitGrid2D {
    let mut grid = BitGrid2D::new(height, width);
    for y in 0..height {
        for x in 0..width {
            let bit_idx = y * width + x;
            if bit_idx < 64 {
                let bit_val = ((val >> bit_idx) & 1) != 0;
                grid.set_cell(y, x, bit_val);
            }
        }
    }
    grid
}

fn grid2d_to_u64(grid: &BitGrid2D) -> u64 {
    let mut val = 0u64;
    for y in 0..grid.height {
        for x in 0..grid.width {
            let bit_idx = y * grid.width + x;
            if bit_idx < 64 && grid.get_cell(y, x) {
                val |= 1u64 << bit_idx;
            }
        }
    }
    val
}

// ─────────────────────────────────────────────────────────────────────
// 1D C-API Functions
// ─────────────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn cahe_keygen_1d(enc_rule: u8, eval_rule: u8, steps: u32, iv: u64) -> CaheKey1D {
    CaheKey1D {
        enc_rule,
        eval_rule,
        steps,
        iv,
    }
}

#[no_mangle]
pub extern "C" fn cahe_encrypt_1d(key: CaheKey1D, plaintext: u64, size: u32) -> CaheCiphertext1D {
    if size == 0 || size > 64 {
        return CaheCiphertext1D { c0: 0, c1: 0 };
    }
    let pt_grid = BitGrid::from_u64(plaintext, size as usize);
    let iv_grid = BitGrid::from_u64(key.iv, size as usize);
    let (c0, c1) = encrypt(&pt_grid, &iv_grid, key.enc_rule, key.steps as usize);
    CaheCiphertext1D {
        c0: c0.to_u64(),
        c1: c1.to_u64(),
    }
}

#[no_mangle]
pub extern "C" fn cahe_decrypt_1d(key: CaheKey1D, ct: CaheCiphertext1D, size: u32) -> u64 {
    if size == 0 || size > 64 {
        return 0;
    }
    let c0_grid = BitGrid::from_u64(ct.c0, size as usize);
    let c1_grid = BitGrid::from_u64(ct.c1, size as usize);
    let iv_grid = BitGrid::from_u64(key.iv, size as usize);
    let pt_grid = decrypt(&c0_grid, &c1_grid, &iv_grid, key.enc_rule, key.steps as usize);
    pt_grid.to_u64()
}

#[no_mangle]
pub extern "C" fn cahe_eval_add_1d(
    key: CaheKey1D,
    ct_a: CaheCiphertext1D,
    ct_b: CaheCiphertext1D,
    size: u32
) -> CaheCiphertext1D {
    if size == 0 || size > 64 {
        return CaheCiphertext1D { c0: 0, c1: 0 };
    }
    // Evolve the XOR of the ciphertexts under the eval rule
    let c_sum0 = BitGrid::from_u64(ct_a.c0 ^ ct_b.c0, size as usize);
    let c_sum1 = BitGrid::from_u64(ct_a.c1 ^ ct_b.c1, size as usize);
    
    let ca_eval = ReversibleCA::new(key.eval_rule, key.steps as usize);
    let (c_eval0, c_eval1) = ca_eval.evolve(&c_sum0, &c_sum1);
    
    CaheCiphertext1D {
        c0: c_eval0.to_u64(),
        c1: c_eval1.to_u64(),
    }
}

#[no_mangle]
pub extern "C" fn cahe_bootstrap_1d(
    _key: CaheKey1D,
    ct: CaheCiphertext1D,
    _size: u32
) -> CaheCiphertext1D {
    // Identity function in leveled FHE PoC
    ct
}

#[no_mangle]
pub extern "C" fn cahe_encode_repetition_1d(val: u64, k: u32, n: u32) -> u64 {
    if n == 0 || n > 64 || k == 0 || k > n {
        return 0;
    }
    let grid = encode_repetition(val, k as usize, n as usize);
    grid.to_u64()
}

#[no_mangle]
pub extern "C" fn cahe_decode_repetition_1d(val: u64, k: u32, n: u32) -> u64 {
    if n == 0 || n > 64 || k == 0 || k > n {
        return 0;
    }
    let grid = BitGrid::from_u64(val, n as usize);
    decode_repetition(&grid, k as usize)
}

// ─────────────────────────────────────────────────────────────────────
// 2D C-API Functions
// ─────────────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn cahe_keygen_2d(enc_rule: u32, eval_rule: u32, steps: u32, iv: u64) -> CaheKey2D {
    CaheKey2D {
        enc_rule,
        eval_rule,
        steps,
        iv,
    }
}

#[no_mangle]
pub extern "C" fn cahe_encrypt_2d(
    key: CaheKey2D,
    plaintext: u64,
    height: u32,
    width: u32
) -> CaheCiphertext2D {
    let h = height as usize;
    let w = width as usize;
    if h == 0 || w == 0 || h * w > 64 {
        return CaheCiphertext2D { c0: 0, c1: 0 };
    }
    let pt_grid = u64_to_grid2d(plaintext, h, w);
    let iv_grid = u64_to_grid2d(key.iv, h, w);
    let rule = CARule2D::VonNeumann(key.enc_rule);
    let (c0, c1) = encrypt_2d(&pt_grid, &iv_grid, &rule, key.steps as usize);
    CaheCiphertext2D {
        c0: grid2d_to_u64(&c0),
        c1: grid2d_to_u64(&c1),
    }
}

#[no_mangle]
pub extern "C" fn cahe_decrypt_2d(
    key: CaheKey2D,
    ct: CaheCiphertext2D,
    height: u32,
    width: u32
) -> u64 {
    let h = height as usize;
    let w = width as usize;
    if h == 0 || w == 0 || h * w > 64 {
        return 0;
    }
    let c0_grid = u64_to_grid2d(ct.c0, h, w);
    let c1_grid = u64_to_grid2d(ct.c1, h, w);
    let iv_grid = u64_to_grid2d(key.iv, h, w);
    let rule = CARule2D::VonNeumann(key.enc_rule);
    let pt_grid = decrypt_2d(&c0_grid, &c1_grid, &iv_grid, &rule, key.steps as usize);
    grid2d_to_u64(&pt_grid)
}

#[no_mangle]
pub extern "C" fn cahe_eval_add_2d(
    key: CaheKey2D,
    ct_a: CaheCiphertext2D,
    ct_b: CaheCiphertext2D,
    height: u32,
    width: u32
) -> CaheCiphertext2D {
    let h = height as usize;
    let w = width as usize;
    if h == 0 || w == 0 || h * w > 64 {
        return CaheCiphertext2D { c0: 0, c1: 0 };
    }
    let c_sum0 = u64_to_grid2d(ct_a.c0 ^ ct_b.c0, h, w);
    let c_sum1 = u64_to_grid2d(ct_a.c1 ^ ct_b.c1, h, w);
    
    let rule_eval = CARule2D::VonNeumann(key.eval_rule);
    let ca_eval = ReversibleCA2D::new(rule_eval, key.steps as usize);
    let (c_eval0, c_eval1) = ca_eval.evolve(&c_sum0, &c_sum1);
    
    CaheCiphertext2D {
        c0: grid2d_to_u64(&c_eval0),
        c1: grid2d_to_u64(&c_eval1),
    }
}

#[no_mangle]
pub extern "C" fn cahe_bootstrap_2d(
    _key: CaheKey2D,
    ct: CaheCiphertext2D,
    _height: u32,
    _width: u32
) -> CaheCiphertext2D {
    // Identity function in leveled FHE PoC
    ct
}

// ─────────────────────────────────────────────────────────────────────
// 3D C-API Functions
// ─────────────────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CaheKey3D {
    pub rule_lut0: u64,
    pub rule_lut1: u64,
    pub steps: u32,
    pub iv: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CaheCiphertext3D {
    pub c0: u64,
    pub c1: u64,
}

// Helper functions for 3D packing
fn u64_to_grid3d(val: u64, depth: usize, height: usize, width: usize) -> BitGrid3D {
    let mut grid = BitGrid3D::new(depth, height, width);
    for z in 0..depth {
        for y in 0..height {
            for x in 0..width {
                let bit_idx = z * (height * width) + y * width + x;
                if bit_idx < 64 {
                    let bit_val = ((val >> bit_idx) & 1) != 0;
                    grid.set_cell(z, y, x, bit_val);
                }
            }
        }
    }
    grid
}

fn grid3d_to_u64(grid: &BitGrid3D) -> u64 {
    let mut val = 0u64;
    for z in 0..grid.depth {
        for y in 0..grid.height {
            for x in 0..grid.width {
                let bit_idx = z * (grid.height * grid.width) + y * grid.width + x;
                if bit_idx < 64 && grid.get_cell(z, y, x) {
                    val |= 1u64 << bit_idx;
                }
            }
        }
    }
    val
}

#[no_mangle]
pub extern "C" fn cahe_keygen_3d(rule_lut0: u64, rule_lut1: u64, steps: u32, iv: u64) -> CaheKey3D {
    CaheKey3D {
        rule_lut0,
        rule_lut1,
        steps,
        iv,
    }
}

#[no_mangle]
pub extern "C" fn cahe_encrypt_3d(
    key: CaheKey3D,
    plaintext: u64,
    depth: u32,
    height: u32,
    width: u32
) -> CaheCiphertext3D {
    let d = depth as usize;
    let h = height as usize;
    let w = width as usize;
    if d == 0 || h == 0 || w == 0 || d * h * w > 64 {
        return CaheCiphertext3D { c0: 0, c1: 0 };
    }
    let pt_grid = u64_to_grid3d(plaintext, d, h, w);
    let iv_grid = u64_to_grid3d(key.iv, d, h, w);
    let rule = CARule3D { lut: [key.rule_lut0, key.rule_lut1] };
    let (c0, c1) = encrypt_3d(&pt_grid, &iv_grid, &rule, key.steps as usize);
    CaheCiphertext3D {
        c0: grid3d_to_u64(&c0),
        c1: grid3d_to_u64(&c1),
    }
}

#[no_mangle]
pub extern "C" fn cahe_decrypt_3d(
    key: CaheKey3D,
    ct: CaheCiphertext3D,
    depth: u32,
    height: u32,
    width: u32
) -> u64 {
    let d = depth as usize;
    let h = height as usize;
    let w = width as usize;
    if d == 0 || h == 0 || w == 0 || d * h * w > 64 {
        return 0;
    }
    let c0_grid = u64_to_grid3d(ct.c0, d, h, w);
    let c1_grid = u64_to_grid3d(ct.c1, d, h, w);
    let iv_grid = u64_to_grid3d(key.iv, d, h, w);
    let rule = CARule3D { lut: [key.rule_lut0, key.rule_lut1] };
    let pt_grid = decrypt_3d(&c0_grid, &c1_grid, &iv_grid, &rule, key.steps as usize);
    grid3d_to_u64(&pt_grid)
}

#[no_mangle]
pub extern "C" fn cahe_eval_add_3d(
    key: CaheKey3D,
    ct_a: CaheCiphertext3D,
    ct_b: CaheCiphertext3D,
    depth: u32,
    height: u32,
    width: u32
) -> CaheCiphertext3D {
    let d = depth as usize;
    let h = height as usize;
    let w = width as usize;
    if d == 0 || h == 0 || w == 0 || d * h * w > 64 {
        return CaheCiphertext3D { c0: 0, c1: 0 };
    }
    let c_sum0 = u64_to_grid3d(ct_a.c0 ^ ct_b.c0, d, h, w);
    let c_sum1 = u64_to_grid3d(ct_a.c1 ^ ct_b.c1, d, h, w);
    
    let rule = CARule3D { lut: [key.rule_lut0, key.rule_lut1] };
    let ca_eval = ReversibleCA3D::new(rule, key.steps as usize);
    let (c_eval0, c_eval1) = ca_eval.evolve(&c_sum0, &c_sum1);
    
    CaheCiphertext3D {
        c0: grid3d_to_u64(&c_eval0),
        c1: grid3d_to_u64(&c_eval1),
    }
}

#[no_mangle]
pub extern "C" fn cahe_bootstrap_3d(
    _key: CaheKey3D,
    ct: CaheCiphertext3D,
    _depth: u32,
    _height: u32,
    _width: u32
) -> CaheCiphertext3D {
    // Identity function in leveled FHE PoC
    ct
}
