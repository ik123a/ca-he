import ctypes
import os
import sys

# Define ctypes Structures matching the Rust #[repr(C)] structs
class CaheKey1D(ctypes.Structure):
    _fields_ = [
        ("enc_rule", ctypes.c_uint8),
        ("eval_rule", ctypes.c_uint8),
        ("steps", ctypes.c_uint32),
        ("iv", ctypes.c_uint64),
    ]

    def __repr__(self):
        return f"CaheKey1D(enc_rule={self.enc_rule}, eval_rule={self.eval_rule}, steps={self.steps}, iv={self.iv:#x})"

class CaheCiphertext1D(ctypes.Structure):
    _fields_ = [
        ("c0", ctypes.c_uint64),
        ("c1", ctypes.c_uint64),
    ]

    def __repr__(self):
        return f"CaheCiphertext1D(c0={self.c0:#x}, c1={self.c1:#x})"

class CaheKey2D(ctypes.Structure):
    _fields_ = [
        ("enc_rule", ctypes.c_uint32),
        ("eval_rule", ctypes.c_uint32),
        ("steps", ctypes.c_uint32),
        ("iv", ctypes.c_uint64),
    ]

    def __repr__(self):
        return f"CaheKey2D(enc_rule={self.enc_rule}, eval_rule={self.eval_rule}, steps={self.steps}, iv={self.iv:#x})"

class CaheCiphertext2D(ctypes.Structure):
    _fields_ = [
        ("c0", ctypes.c_uint64),
        ("c1", ctypes.c_uint64),
    ]

    def __repr__(self):
        return f"CaheCiphertext2D(c0={self.c0:#x}, c1={self.c1:#x})"


def load_library():
    lib_names = ["ca_he_core.dll", "libca_he_core.so", "libca_he_core.dylib"]
    
    # Try searching relative directories first
    search_dirs = [
        os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "rust", "target", "release")),
        os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "rust", "target", "debug")),
        os.path.abspath(os.path.dirname(__file__)),
        os.getcwd(),
    ]
    
    for search_dir in search_dirs:
        for lib_name in lib_names:
            path = os.path.join(search_dir, lib_name)
            if os.path.exists(path):
                try:
                    return ctypes.CDLL(path)
                except Exception as e:
                    print(f"Failed to load {path}: {e}")
                    
    # Fallback to system search paths
    for lib_name in lib_names:
        try:
            return ctypes.CDLL(lib_name)
        except Exception:
            continue
            
    raise OSError(
        "Could not find or load ca_he_core dynamic library.\n"
        f"Searched directories: {search_dirs}"
    )


# Load the shared library
_lib = load_library()

# ─────────────────────────────────────────────────────────────────────
# 1D API Bindings
# ─────────────────────────────────────────────────────────────────────

_lib.cahe_keygen_1d.argtypes = [ctypes.c_uint8, ctypes.c_uint8, ctypes.c_uint32, ctypes.c_uint64]
_lib.cahe_keygen_1d.restype = CaheKey1D

_lib.cahe_encrypt_1d.argtypes = [CaheKey1D, ctypes.c_uint64, ctypes.c_uint32]
_lib.cahe_encrypt_1d.restype = CaheCiphertext1D

_lib.cahe_decrypt_1d.argtypes = [CaheKey1D, CaheCiphertext1D, ctypes.c_uint32]
_lib.cahe_decrypt_1d.restype = ctypes.c_uint64

_lib.cahe_eval_add_1d.argtypes = [CaheKey1D, CaheCiphertext1D, CaheCiphertext1D, ctypes.c_uint32]
_lib.cahe_eval_add_1d.restype = CaheCiphertext1D

_lib.cahe_bootstrap_1d.argtypes = [CaheKey1D, CaheCiphertext1D, ctypes.c_uint32]
_lib.cahe_bootstrap_1d.restype = CaheCiphertext1D

_lib.cahe_encode_repetition_1d.argtypes = [ctypes.c_uint64, ctypes.c_uint32, ctypes.c_uint32]
_lib.cahe_encode_repetition_1d.restype = ctypes.c_uint64

_lib.cahe_decode_repetition_1d.argtypes = [ctypes.c_uint64, ctypes.c_uint32, ctypes.c_uint32]
_lib.cahe_decode_repetition_1d.restype = ctypes.c_uint64


def keygen_1d(enc_rule: int, eval_rule: int, steps: int, iv: int) -> CaheKey1D:
    return _lib.cahe_keygen_1d(enc_rule, eval_rule, steps, iv)

def encrypt_1d(key: CaheKey1D, plaintext: int, size: int) -> CaheCiphertext1D:
    return _lib.cahe_encrypt_1d(key, plaintext, size)

def decrypt_1d(key: CaheKey1D, ct: CaheCiphertext1D, size: int) -> int:
    return _lib.cahe_decrypt_1d(key, ct, size)

def eval_add_1d(key: CaheKey1D, ct_a: CaheCiphertext1D, ct_b: CaheCiphertext1D, size: int) -> CaheCiphertext1D:
    return _lib.cahe_eval_add_1d(key, ct_a, ct_b, size)

def bootstrap_1d(key: CaheKey1D, ct: CaheCiphertext1D, size: int) -> CaheCiphertext1D:
    return _lib.cahe_bootstrap_1d(key, ct, size)

def encode_repetition_1d(val: int, k: int, n: int) -> int:
    return _lib.cahe_encode_repetition_1d(val, k, n)

def decode_repetition_1d(val: int, k: int, n: int) -> int:
    return _lib.cahe_decode_repetition_1d(val, k, n)


# ─────────────────────────────────────────────────────────────────────
# 2D API Bindings
# ─────────────────────────────────────────────────────────────────────

_lib.cahe_keygen_2d.argtypes = [ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint64]
_lib.cahe_keygen_2d.restype = CaheKey2D

_lib.cahe_encrypt_2d.argtypes = [CaheKey2D, ctypes.c_uint64, ctypes.c_uint32, ctypes.c_uint32]
_lib.cahe_encrypt_2d.restype = CaheCiphertext2D

_lib.cahe_decrypt_2d.argtypes = [CaheKey2D, CaheCiphertext2D, ctypes.c_uint32, ctypes.c_uint32]
_lib.cahe_decrypt_2d.restype = ctypes.c_uint64

_lib.cahe_eval_add_2d.argtypes = [CaheKey2D, CaheCiphertext2D, CaheCiphertext2D, ctypes.c_uint32, ctypes.c_uint32]
_lib.cahe_eval_add_2d.restype = CaheCiphertext2D

_lib.cahe_bootstrap_2d.argtypes = [CaheKey2D, CaheCiphertext2D, ctypes.c_uint32, ctypes.c_uint32]
_lib.cahe_bootstrap_2d.restype = CaheCiphertext2D


def keygen_2d(enc_rule: int, eval_rule: int, steps: int, iv: int) -> CaheKey2D:
    return _lib.cahe_keygen_2d(enc_rule, eval_rule, steps, iv)

def encrypt_2d(key: CaheKey2D, plaintext: int, height: int, width: int) -> CaheCiphertext2D:
    return _lib.cahe_encrypt_2d(key, plaintext, height, width)

def decrypt_2d(key: CaheKey2D, ct: CaheCiphertext2D, height: int, width: int) -> int:
    return _lib.cahe_decrypt_2d(key, ct, height, width)

def eval_add_2d(key: CaheKey2D, ct_a: CaheCiphertext2D, ct_b: CaheCiphertext2D, height: int, width: int) -> CaheCiphertext2D:
    return _lib.cahe_eval_add_2d(key, ct_a, ct_b, height, width)

def bootstrap_2d(key: CaheKey2D, ct: CaheCiphertext2D, height: int, width: int) -> CaheCiphertext2D:
    return _lib.cahe_bootstrap_2d(key, ct, height, width)
