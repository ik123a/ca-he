"""
CA-HE Evolutionary Search: Multi-Objective Rule Pair Discovery

Uses NSGA-II-inspired multi-objective optimization to find CA rule pairs
(enc_rule, eval_rule) that maximize:
  F1: Homomorphic accuracy (XOR and/or addition)
  F2: Avalanche/diffusion quality
  F3: Nonlinearity (quantum resistance proxy)
  F4: Computational efficiency

Designed to scale beyond the exhaustive search to larger grids (2D, higher radius).
"""

import sys
import os
import random
import time
import json
import math
from dataclasses import dataclass, field
from typing import List, Tuple, Optional
from copy import deepcopy

sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'src'))
from ca_core import (apply_rule_1d, evolve_reversible, reverse_evolve,
                     encrypt, decrypt)


# ─────────────────────────────────────────────────────────────────────
# Genome Representation
# ─────────────────────────────────────────────────────────────────────

@dataclass
class RulePairGenome:
    """
    Encodes a CA rule pair (encryption rule + evaluation rule) as an
    evolvable genome.
    """
    enc_lut: int          # 8-bit Wolfram number for encryption rule
    eval_lut: int         # 8-bit Wolfram number for evaluation rule
    steps: int            # Number of evolution steps
    grid_size: int        # Grid size N
    
    # Fitness values (filled during evaluation)
    fitness: dict = field(default_factory=dict)
    rank: int = 0         # Pareto rank (0 = non-dominated)
    crowding: float = 0.0 # Crowding distance
    
    def copy(self):
        g = RulePairGenome(
            enc_lut=self.enc_lut,
            eval_lut=self.eval_lut,
            steps=self.steps,
            grid_size=self.grid_size,
        )
        g.fitness = dict(self.fitness)
        g.rank = self.rank
        g.crowding = self.crowding
        return g


# ─────────────────────────────────────────────────────────────────────
# Fitness Functions
# ─────────────────────────────────────────────────────────────────────

def is_linear_rule(rule_lut: int) -> bool:
    """Check if a rule is affine/linear over GF(2)."""
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


def nonlinearity_score(rule_lut: int) -> float:
    """
    Compute the nonlinearity of a 3-input Boolean function.
    
    Nonlinearity = minimum Hamming distance to any affine function.
    Maximum possible for 3 inputs: 2 (out of 8 entries).
    Returns normalized score in [0, 1].
    """
    min_dist = 8
    for a0 in range(2):
        for a1 in range(2):
            for a2 in range(2):
                for a3 in range(2):
                    dist = 0
                    for idx in range(8):
                        x0 = (idx >> 0) & 1
                        x1 = (idx >> 1) & 1
                        x2 = (idx >> 2) & 1
                        affine = (a3 * x2) ^ (a2 * x1) ^ (a1 * x0) ^ a0
                        actual = (rule_lut >> idx) & 1
                        if affine != actual:
                            dist += 1
                    min_dist = min(min_dist, dist)
    
    # Max nonlinearity for 3-input function is 2
    return min_dist / 2.0 if min_dist <= 2 else 1.0


def evaluate_fitness(genome: RulePairGenome, test_pairs: List[Tuple[int, int]],
                     iv: int = 0) -> dict:
    """
    Evaluate all fitness objectives for a genome.
    
    Returns dict with keys: 'homo_xor', 'homo_add', 'avalanche', 
                            'nonlinearity', 'cost'
    """
    n = genome.grid_size
    mask = (1 << n) - 1
    steps = genome.steps
    enc = genome.enc_lut
    eva = genome.eval_lut
    
    # ── F1: Homomorphic Accuracy ──
    xor_correct = 0
    add_correct = 0
    
    # Pre-compute encryptions
    encrypted = {}
    for a, b in test_pairs:
        if a not in encrypted:
            encrypted[a] = encrypt(a, iv, enc, n, steps)
        if b not in encrypted:
            encrypted[b] = encrypt(b, iv, enc, n, steps)
    
    for a, b in test_pairs:
        c_a = encrypted[a]
        c_b = encrypted[b]
        combined = (c_a[0] ^ c_b[0], c_a[1] ^ c_b[1])
        c_eval = evolve_reversible(combined[0], combined[1], eva, n, steps)
        result = decrypt(c_eval[0], c_eval[1], iv, enc, n, steps)
        
        if result == (a ^ b):
            xor_correct += 1
        if result == ((a + b) & mask):
            add_correct += 1
    
    homo_xor = xor_correct / len(test_pairs)
    homo_add = add_correct / len(test_pairs)
    
    # ── F2: Avalanche Score ──
    total_hd = 0
    avalanche_tests = min(20, len(test_pairs))
    for i in range(avalanche_tests):
        m = test_pairs[i][0]
        c = encrypt(m, iv, enc, n, steps)
        
        # Flip one random bit
        pos = i % n
        m_flipped = m ^ (1 << pos)
        c_flipped = encrypt(m_flipped, iv, enc, n, steps)
        
        # Hamming distance of ciphertexts
        hd0 = bin(c[0] ^ c_flipped[0]).count('1')
        hd1 = bin(c[1] ^ c_flipped[1]).count('1')
        total_hd += (hd0 + hd1) / (2 * n)
    
    avalanche = total_hd / avalanche_tests  # Target: 0.5
    avalanche_score = 1.0 - abs(avalanche - 0.5) / 0.5
    
    # ── F3: Nonlinearity ──
    nl_enc = nonlinearity_score(enc)
    nl_eval = nonlinearity_score(eva)
    nl_combined = (nl_enc + nl_eval) / 2.0
    
    # ── F4: Cost (inverse of steps, lower is better) ──
    cost_score = max(0, 1.0 - steps / 128.0)
    
    return {
        'homo_xor': homo_xor,
        'homo_add': homo_add,
        'avalanche': avalanche_score,
        'nonlinearity': nl_combined,
        'cost': cost_score,
        # Weighted aggregate for single-objective fallback
        'aggregate': (0.4 * max(homo_xor, homo_add) + 
                      0.2 * avalanche_score + 
                      0.25 * nl_combined + 
                      0.15 * cost_score),
    }


# ─────────────────────────────────────────────────────────────────────
# NSGA-II Selection
# ─────────────────────────────────────────────────────────────────────

def dominates(a: dict, b: dict, objectives=('homo_xor', 'avalanche', 'nonlinearity')) -> bool:
    """Return True if fitness `a` Pareto-dominates fitness `b`."""
    at_least_one_better = False
    for obj in objectives:
        if a[obj] < b[obj]:
            return False
        if a[obj] > b[obj]:
            at_least_one_better = True
    return at_least_one_better


def non_dominated_sort(population: List[RulePairGenome], 
                       objectives=('homo_xor', 'avalanche', 'nonlinearity')):
    """Assign Pareto ranks to the population."""
    n = len(population)
    domination_count = [0] * n
    dominated_set = [[] for _ in range(n)]
    fronts = [[]]
    
    for i in range(n):
        for j in range(i + 1, n):
            if dominates(population[i].fitness, population[j].fitness, objectives):
                dominated_set[i].append(j)
                domination_count[j] += 1
            elif dominates(population[j].fitness, population[i].fitness, objectives):
                dominated_set[j].append(i)
                domination_count[i] += 1
        
        if domination_count[i] == 0:
            population[i].rank = 0
            fronts[0].append(i)
    
    front_idx = 0
    while fronts[front_idx]:
        next_front = []
        for i in fronts[front_idx]:
            for j in dominated_set[i]:
                domination_count[j] -= 1
                if domination_count[j] == 0:
                    population[j].rank = front_idx + 1
                    next_front.append(j)
        front_idx += 1
        fronts.append(next_front)
    
    return fronts[:-1]  # Remove last empty front


def crowding_distance(population: List[RulePairGenome], front: List[int],
                      objectives=('homo_xor', 'avalanche', 'nonlinearity')):
    """Compute crowding distance for individuals in a Pareto front."""
    if len(front) <= 2:
        for i in front:
            population[i].crowding = float('inf')
        return
    
    for i in front:
        population[i].crowding = 0.0
    
    for obj in objectives:
        sorted_front = sorted(front, key=lambda i: population[i].fitness.get(obj, 0))
        population[sorted_front[0]].crowding = float('inf')
        population[sorted_front[-1]].crowding = float('inf')
        
        obj_range = (population[sorted_front[-1]].fitness.get(obj, 0) - 
                     population[sorted_front[0]].fitness.get(obj, 0))
        if obj_range == 0:
            continue
        
        for k in range(1, len(sorted_front) - 1):
            population[sorted_front[k]].crowding += (
                (population[sorted_front[k+1]].fitness.get(obj, 0) - 
                 population[sorted_front[k-1]].fitness.get(obj, 0)) / obj_range
            )


def tournament_select(population: List[RulePairGenome], 
                      tournament_size: int = 3) -> RulePairGenome:
    """Binary tournament selection based on rank and crowding distance."""
    candidates = random.sample(range(len(population)), 
                              min(tournament_size, len(population)))
    best = candidates[0]
    for c in candidates[1:]:
        if (population[c].rank < population[best].rank or
            (population[c].rank == population[best].rank and 
             population[c].crowding > population[best].crowding)):
            best = c
    return population[best].copy()


# ─────────────────────────────────────────────────────────────────────
# Genetic Operators
# ─────────────────────────────────────────────────────────────────────

def crossover(parent1: RulePairGenome, parent2: RulePairGenome) -> RulePairGenome:
    """Uniform crossover on LUT bits."""
    child = parent1.copy()
    
    # Crossover enc_lut bits
    crossover_mask = random.randint(0, 255)
    child.enc_lut = (parent1.enc_lut & crossover_mask) | (parent2.enc_lut & ~crossover_mask & 0xFF)
    
    # Crossover eval_lut bits
    crossover_mask = random.randint(0, 255)
    child.eval_lut = (parent1.eval_lut & crossover_mask) | (parent2.eval_lut & ~crossover_mask & 0xFF)
    
    # Average steps with noise
    child.steps = max(4, min(128, (parent1.steps + parent2.steps) // 2 + random.randint(-2, 2)))
    
    return child


def mutate(genome: RulePairGenome, mutation_rate: float = 0.15) -> RulePairGenome:
    """Apply mutation operators."""
    g = genome.copy()
    
    if random.random() < mutation_rate:
        # Flip a random bit in enc_lut
        bit = random.randint(0, 7)
        g.enc_lut ^= (1 << bit)
    
    if random.random() < mutation_rate:
        # Flip a random bit in eval_lut
        bit = random.randint(0, 7)
        g.eval_lut ^= (1 << bit)
    
    if random.random() < mutation_rate * 0.5:
        # Perturb step count
        g.steps = max(4, min(128, g.steps + random.randint(-4, 4)))
    
    if random.random() < mutation_rate * 0.3:
        # Swap two LUT entries in enc (preserves Hamming weight)
        bit1, bit2 = random.sample(range(8), 2)
        v1 = (g.enc_lut >> bit1) & 1
        v2 = (g.enc_lut >> bit2) & 1
        if v1 != v2:
            g.enc_lut ^= (1 << bit1) | (1 << bit2)
    
    return g


def random_genome(grid_size: int = 8, steps_range: Tuple[int, int] = (8, 64)) -> RulePairGenome:
    """Create a random genome, biased toward nonlinear rules."""
    while True:
        enc = random.randint(0, 255)
        if not is_linear_rule(enc):  # Prefer nonlinear encryption rules
            break
    
    eval_lut = random.randint(0, 255)
    steps = random.randint(steps_range[0], steps_range[1])
    
    return RulePairGenome(
        enc_lut=enc,
        eval_lut=eval_lut,
        steps=steps,
        grid_size=grid_size,
    )


# ─────────────────────────────────────────────────────────────────────
# Main Evolutionary Loop
# ─────────────────────────────────────────────────────────────────────

def evolutionary_search(
    grid_size: int = 8,
    population_size: int = 100,
    generations: int = 200,
    num_test_pairs: int = 30,
    crossover_rate: float = 0.7,
    mutation_rate: float = 0.15,
    seed: int = 42,
    verbose: bool = True,
):
    """
    Run NSGA-II evolutionary search for homomorphic CA rule pairs.
    """
    random.seed(seed)
    mask = (1 << grid_size) - 1
    iv = 0
    
    # Generate test pairs
    test_pairs = [(random.randint(0, mask), random.randint(0, mask)) 
                  for _ in range(num_test_pairs)]
    
    # Initialize population (biased toward nonlinear rules)
    population = [random_genome(grid_size) for _ in range(population_size)]
    
    # Also seed with some known-interesting rules
    # (Include a few linear rules as reference points)
    for rule in [90, 150, 60, 102]:  # Known linear/additive rules
        g = RulePairGenome(enc_lut=rule, eval_lut=rule, 
                          steps=16, grid_size=grid_size)
        population.append(g)
    
    population = population[:population_size]
    
    best_ever = None
    best_fitness = 0.0
    stagnation_counter = 0
    
    print("=" * 70)
    print(f"EVOLUTIONARY SEARCH (NSGA-II)")
    print(f"=" * 70)
    print(f"Grid: {grid_size}, Pop: {population_size}, Gens: {generations}")
    print(f"Test pairs: {num_test_pairs}, Crossover: {crossover_rate}, Mutation: {mutation_rate}")
    print()
    
    start_time = time.time()
    
    for gen in range(generations):
        # Evaluate fitness
        for ind in population:
            if not ind.fitness:
                ind.fitness = evaluate_fitness(ind, test_pairs, iv)
        
        # Non-dominated sorting
        fronts = non_dominated_sort(population)
        
        # Crowding distance
        for front in fronts:
            crowding_distance(population, front)
        
        # Track best
        current_best = max(population, key=lambda g: g.fitness.get('aggregate', 0))
        if current_best.fitness.get('aggregate', 0) > best_fitness:
            best_fitness = current_best.fitness['aggregate']
            best_ever = current_best.copy()
            stagnation_counter = 0
        else:
            stagnation_counter += 1
        
        # Print progress
        if verbose and (gen % 20 == 0 or gen == generations - 1):
            front0_size = len(fronts[0]) if fronts else 0
            avg_homo = sum(g.fitness.get('homo_xor', 0) for g in population) / len(population)
            best_homo = max(g.fitness.get('homo_xor', 0) for g in population)
            best_nl = max(g.fitness.get('nonlinearity', 0) for g in population)
            elapsed = time.time() - start_time
            
            print(f"Gen {gen:4d} | "
                  f"Front0: {front0_size:3d} | "
                  f"Best homo: {best_homo:.3f} | "
                  f"Avg homo: {avg_homo:.3f} | "
                  f"Best NL: {best_nl:.2f} | "
                  f"Best agg: {best_fitness:.3f} | "
                  f"{elapsed:.1f}s")
        
        # Early termination: check if any single individual meets both criteria
        converged = False
        for g in population:
            if g.fitness.get('homo_xor', 0) >= 0.95 and not is_linear_rule(g.enc_lut):
                print(f"\n* CONVERGENCE: Found high-quality NONLINEAR rule pair at gen {gen}!")
                print(f"  Enc rule: {g.enc_lut}, Eval rule: {g.eval_lut}, Homo_XOR: {g.fitness.get('homo_xor', 0)}")
                converged = True
                break
        if converged:
            break
        
        if stagnation_counter > 100:
            if verbose:
                print(f"  ⚠ Stagnation detected at gen {gen}, injecting diversity")
            # Inject fresh random individuals
            for i in range(population_size // 4):
                population[-(i+1)] = random_genome(grid_size)
                population[-(i+1)].fitness = evaluate_fitness(
                    population[-(i+1)], test_pairs, iv)
            stagnation_counter = 0
        
        # Create offspring
        offspring = []
        while len(offspring) < population_size:
            if random.random() < crossover_rate:
                p1 = tournament_select(population)
                p2 = tournament_select(population)
                child = crossover(p1, p2)
            else:
                child = tournament_select(population)
            
            child = mutate(child, mutation_rate)
            child.fitness = {}  # Will be evaluated next generation
            offspring.append(child)
        
        # (μ + λ) selection: combine parents + offspring, keep best
        combined = population + offspring
        for ind in combined:
            if not ind.fitness:
                ind.fitness = evaluate_fitness(ind, test_pairs, iv)
        
        fronts = non_dominated_sort(combined)
        for front in fronts:
            crowding_distance(combined, front)
        
        # Select top population_size by rank, then crowding distance
        combined.sort(key=lambda g: (g.rank, -g.crowding))
        population = combined[:population_size]
    
    total_time = time.time() - start_time
    
    # ─── Final Results ───
    print()
    print("=" * 70)
    print("EVOLUTIONARY SEARCH RESULTS")
    print("=" * 70)
    print(f"Total time: {total_time:.2f}s")
    print(f"Best aggregate fitness: {best_fitness:.4f}")
    
    if best_ever:
        print(f"\nBest individual:")
        print(f"  Enc rule: {best_ever.enc_lut} (0b{best_ever.enc_lut:08b})")
        print(f"  Eval rule: {best_ever.eval_lut} (0b{best_ever.eval_lut:08b})")
        print(f"  Steps: {best_ever.steps}")
        print(f"  Enc linear: {is_linear_rule(best_ever.enc_lut)}")
        print(f"  Fitness:")
        for k, v in best_ever.fitness.items():
            print(f"    {k}: {v:.4f}")
    
    # Show Pareto front
    front0 = [g for g in population if g.rank == 0]
    print(f"\nPareto front (rank 0): {len(front0)} individuals")
    
    # Sort front by homo_xor
    front0.sort(key=lambda g: -g.fitness.get('homo_xor', 0))
    print(f"{'Enc':>5} {'Eval':>5} {'Steps':>5} {'Homo_XOR':>9} {'Homo_ADD':>9} "
          f"{'Avalnch':>8} {'NonLin':>7} {'Agg':>7} {'LinEnc':>6}")
    print("-" * 80)
    for g in front0[:15]:
        f = g.fitness
        lin = "L" if is_linear_rule(g.enc_lut) else "N"
        print(f"{g.enc_lut:5d} {g.eval_lut:5d} {g.steps:5d} "
              f"{f.get('homo_xor',0):9.4f} {f.get('homo_add',0):9.4f} "
              f"{f.get('avalanche',0):8.4f} {f.get('nonlinearity',0):7.4f} "
              f"{f.get('aggregate',0):7.4f} {lin:>6}")
    
    # Save results
    results = {
        'params': {
            'grid_size': grid_size,
            'population_size': population_size,
            'generations': generations,
            'num_test_pairs': num_test_pairs,
        },
        'time_seconds': total_time,
        'best_ever': {
            'enc_lut': best_ever.enc_lut if best_ever else None,
            'eval_lut': best_ever.eval_lut if best_ever else None,
            'steps': best_ever.steps if best_ever else None,
            'fitness': best_ever.fitness if best_ever else None,
            'enc_is_linear': is_linear_rule(best_ever.enc_lut) if best_ever else None,
        },
        'pareto_front': [
            {
                'enc_lut': g.enc_lut,
                'eval_lut': g.eval_lut,
                'steps': g.steps,
                'fitness': g.fitness,
                'enc_is_linear': is_linear_rule(g.enc_lut),
            }
            for g in front0
        ],
    }
    
    results_path = os.path.join(os.path.dirname(__file__), 'results', 
                                'evolutionary_search_results.json')
    os.makedirs(os.path.dirname(results_path), exist_ok=True)
    with open(results_path, 'w') as f:
        json.dump(results, f, indent=2)
    print(f"\nResults saved to {results_path}")
    
    return results


def best_homo_in_pop(population):
    return max(g.fitness.get('homo_xor', 0) for g in population)

def best_nl_in_pop(population):
    return max(g.fitness.get('nonlinearity', 0) for g in population)


if __name__ == '__main__':
    # Run evolutionary search
    evolutionary_search(
        grid_size=8,
        population_size=100,
        generations=200,
        num_test_pairs=30,
        seed=42,
    )
