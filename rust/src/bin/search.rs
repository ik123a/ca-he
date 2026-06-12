use ca_he_core::{encrypt, decrypt, encode_repetition, decode_repetition, BitGrid, ReversibleCA};
use rand::Rng;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct Fitness {
    pub homo_xor: f64,
    pub homo_add: f64,
    pub avalanche: f64,
    pub nonlinearity: f64,
    pub cost: f64,
    pub aggregate: f64,
}

#[derive(Clone, Debug)]
pub struct Genome {
    pub enc_lut: u8,
    pub eval_lut: u8,
    pub steps: usize,
    pub grid_size: usize,
    pub fitness: Fitness,
    pub rank: usize,
    pub crowding: f64,
}

impl Default for Fitness {
    fn default() -> Self {
        Fitness {
            homo_xor: 0.0,
            homo_add: 0.0,
            avalanche: 0.0,
            nonlinearity: 0.0,
            cost: 0.0,
            aggregate: 0.0,
        }
    }
}

pub fn is_linear_rule(rule_lut: u8) -> bool {
    for a0 in 0..2 {
        for a1 in 0..2 {
            for a2 in 0..2 {
                for a3 in 0..2 {
                    let mut match_found = true;
                    for idx in 0..8 {
                        let x0 = (idx >> 0) & 1;
                        let x1 = (idx >> 1) & 1;
                        let x2 = (idx >> 2) & 1;
                        let expected = (a3 * x2) ^ (a2 * x1) ^ (a1 * x0) ^ a0;
                        let actual = ((rule_lut >> idx) & 1) as i32;
                        if expected != actual {
                            match_found = false;
                            break;
                        }
                    }
                    if match_found {
                        return true;
                    }
                }
            }
        }
    }
    false
}

pub fn nonlinearity_score(rule_lut: u8) -> f64 {
    let mut min_dist = 8;
    for a0 in 0..2 {
        for a1 in 0..2 {
            for a2 in 0..2 {
                for a3 in 0..2 {
                    let mut dist = 0;
                    for idx in 0..8 {
                        let x0 = (idx >> 0) & 1;
                        let x1 = (idx >> 1) & 1;
                        let x2 = (idx >> 2) & 1;
                        let affine = (a3 * x2) ^ (a2 * x1) ^ (a1 * x0) ^ a0;
                        let actual = ((rule_lut >> idx) & 1) as i32;
                        if affine != actual {
                            dist += 1;
                        }
                    }
                    if dist < min_dist {
                        min_dist = dist;
                    }
                }
            }
        }
    }
    if min_dist <= 2 {
        min_dist as f64 / 2.0
    } else {
        1.0
    }
}

pub fn evaluate_fitness(
    enc_lut: u8,
    eval_lut: u8,
    steps: usize,
    grid_size: usize,
    k_bits: usize,
    xor_pairs: &[(BitGrid, BitGrid, u64, u64)],
    add_pairs: &[(BitGrid, BitGrid, u64, u64)],
) -> Fitness {
    let iv = BitGrid::new(grid_size);
    let k_mask = if k_bits == 64 { u64::MAX } else { (1u64 << k_bits) - 1 };

    // 1. XOR Homomorphism Accuracy
    let mut xor_correct = 0;
    for (grid_a, grid_b, val_a, val_b) in xor_pairs {
        let (c_a0, c_a1) = encrypt(grid_a, &iv, enc_lut, steps);
        let (c_b0, c_b1) = encrypt(grid_b, &iv, enc_lut, steps);

        let c_sum0 = &c_a0 ^ &c_b0;
        let c_sum1 = &c_a1 ^ &c_b1;

        let ca_eval = ReversibleCA::new(eval_lut, steps);
        let (c_eval0, c_eval1) = ca_eval.evolve(&c_sum0, &c_sum1);

        let dec = decrypt(&c_eval0, &c_eval1, &iv, enc_lut, steps);
        let decoded_val = decode_repetition(&dec, k_bits);

        if decoded_val == (val_a ^ val_b) & k_mask {
            xor_correct += 1;
        }
    }
    let homo_xor = xor_correct as f64 / xor_pairs.len() as f64;

    // 2. Addition Homomorphism Accuracy
    let mut add_correct = 0;
    for (grid_a, grid_b, val_a, val_b) in add_pairs {
        let (c_a0, c_a1) = encrypt(grid_a, &iv, enc_lut, steps);
        let (c_b0, c_b1) = encrypt(grid_b, &iv, enc_lut, steps);

        let c_sum0 = &c_a0 ^ &c_b0;
        let c_sum1 = &c_a1 ^ &c_b1;

        let ca_eval = ReversibleCA::new(eval_lut, steps);
        let (c_eval0, c_eval1) = ca_eval.evolve(&c_sum0, &c_sum1);

        let dec = decrypt(&c_eval0, &c_eval1, &iv, enc_lut, steps);
        let decoded_val = decode_repetition(&dec, k_bits);

        if decoded_val == val_a.wrapping_add(*val_b) & k_mask {
            add_correct += 1;
        }
    }
    let homo_add = add_correct as f64 / add_pairs.len() as f64;

    // 3. Avalanche Score
    let mut total_hd = 0.0;
    let avalanche_tests = std::cmp::min(20, xor_pairs.len());
    for i in 0..avalanche_tests {
        let (grid_m, _, val_m, _) = &xor_pairs[i];
        let c = encrypt(grid_m, &iv, enc_lut, steps);

        // Flip one bit in the plaintext value
        let pos = i % k_bits;
        let val_flipped = val_m ^ (1u64 << pos);
        let grid_flipped = encode_repetition(val_flipped, k_bits, grid_size);
        let c_flipped = encrypt(&grid_flipped, &iv, enc_lut, steps);

        let hd0 = (c.0 ^ c_flipped.0).to_bits().iter().map(|&x| x as f64).sum::<f64>();
        let hd1 = (c.1 ^ c_flipped.1).to_bits().iter().map(|&x| x as f64).sum::<f64>();
        total_hd += (hd0 + hd1) / (2.0 * grid_size as f64);
    }
    let avalanche = total_hd / avalanche_tests as f64;
    let avalanche_score = 1.0 - (avalanche - 0.5).abs() / 0.5;

    // 4. Nonlinearity
    let nl_enc = nonlinearity_score(enc_lut);
    let nl_eval = nonlinearity_score(eval_lut);
    let nl_combined = (nl_enc + nl_eval) / 2.0;

    // 5. Cost
    let cost_score = 1.0 - (steps as f64 / 128.0);
    let cost_score = if cost_score < 0.0 { 0.0 } else { cost_score };

    let max_homo = homo_xor.max(homo_add);
    let aggregate = 0.4 * max_homo + 0.2 * avalanche_score + 0.25 * nl_combined + 0.15 * cost_score;

    Fitness {
        homo_xor,
        homo_add,
        avalanche: avalanche_score,
        nonlinearity: nl_combined,
        cost: cost_score,
        aggregate,
    }
}

fn dominates(a: &Fitness, b: &Fitness) -> bool {
    let a_homo = a.homo_xor.max(a.homo_add);
    let b_homo = b.homo_xor.max(b.homo_add);

    let at_least_one_better = (a_homo > b_homo && a.avalanche >= b.avalanche && a.nonlinearity >= b.nonlinearity)
        || (a_homo >= b_homo && a.avalanche > b.avalanche && a.nonlinearity >= b.nonlinearity)
        || (a_homo >= b_homo && a.avalanche >= b.avalanche && a.nonlinearity > b.nonlinearity);

    let none_worse = a_homo >= b_homo && a.avalanche >= b.avalanche && a.nonlinearity >= b.nonlinearity;

    none_worse && at_least_one_better
}

fn non_dominated_sort(population: &mut [Genome]) -> Vec<Vec<usize>> {
    let n = population.len();
    let mut domination_count = vec![0; n];
    let mut dominated_set = vec![vec![]; n];
    let mut fronts = vec![vec![]];

    for i in 0..n {
        for j in 0..n {
            if i == j { continue; }
            if dominates(&population[i].fitness, &population[j].fitness) {
                dominated_set[i].push(j);
            } else if dominates(&population[j].fitness, &population[i].fitness) {
                domination_count[i] += 1;
            }
        }
        if domination_count[i] == 0 {
            population[i].rank = 0;
            fronts[0].push(i);
        }
    }

    let mut front_idx = 0;
    while !fronts[front_idx].is_empty() {
        let mut next_front = vec![];
        for &i in &fronts[front_idx] {
            for &j in &dominated_set[i] {
                domination_count[j] -= 1;
                if domination_count[j] == 0 {
                    population[j].rank = front_idx + 1;
                    next_front.push(j);
                }
            }
        }
        front_idx += 1;
        fronts.push(next_front);
    }
    fronts.pop();
    fronts
}

fn calculate_crowding_distance(population: &mut [Genome], front: &[usize]) {
    if front.len() <= 2 {
        for &i in front {
            population[i].crowding = f64::INFINITY;
        }
        return;
    }

    for &i in front {
        population[i].crowding = 0.0;
    }

    // Obj 0: max(homo_xor, homo_add)
    let mut sorted_indices = front.to_vec();
    sorted_indices.sort_by(|&a, &b| {
        let val_a = population[a].fitness.homo_xor.max(population[a].fitness.homo_add);
        let val_b = population[b].fitness.homo_xor.max(population[b].fitness.homo_add);
        val_a.partial_cmp(&val_b).unwrap()
    });
    population[sorted_indices[0]].crowding = f64::INFINITY;
    population[*sorted_indices.last().unwrap()].crowding = f64::INFINITY;
    let min_val = population[sorted_indices[0]].fitness.homo_xor.max(population[sorted_indices[0]].fitness.homo_add);
    let max_val = population[*sorted_indices.last().unwrap()].fitness.homo_xor.max(population[*sorted_indices.last().unwrap()].fitness.homo_add);
    let range = max_val - min_val;
    if range > 0.0 {
        for k in 1..sorted_indices.len() - 1 {
            let val_next = population[sorted_indices[k+1]].fitness.homo_xor.max(population[sorted_indices[k+1]].fitness.homo_add);
            let val_prev = population[sorted_indices[k-1]].fitness.homo_xor.max(population[sorted_indices[k-1]].fitness.homo_add);
            population[sorted_indices[k]].crowding += (val_next - val_prev) / range;
        }
    }

    // Obj 1: avalanche
    sorted_indices.sort_by(|&a, &b| population[a].fitness.avalanche.partial_cmp(&population[b].fitness.avalanche).unwrap());
    population[sorted_indices[0]].crowding = f64::INFINITY;
    population[*sorted_indices.last().unwrap()].crowding = f64::INFINITY;
    let range = population[*sorted_indices.last().unwrap()].fitness.avalanche - population[sorted_indices[0]].fitness.avalanche;
    if range > 0.0 {
        for k in 1..sorted_indices.len() - 1 {
            population[sorted_indices[k]].crowding += (population[sorted_indices[k+1]].fitness.avalanche - population[sorted_indices[k-1]].fitness.avalanche) / range;
        }
    }

    // Obj 2: nonlinearity
    sorted_indices.sort_by(|&a, &b| population[a].fitness.nonlinearity.partial_cmp(&population[b].fitness.nonlinearity).unwrap());
    population[sorted_indices[0]].crowding = f64::INFINITY;
    population[*sorted_indices.last().unwrap()].crowding = f64::INFINITY;
    let range = population[*sorted_indices.last().unwrap()].fitness.nonlinearity - population[sorted_indices[0]].fitness.nonlinearity;
    if range > 0.0 {
        for k in 1..sorted_indices.len() - 1 {
            population[sorted_indices[k]].crowding += (population[sorted_indices[k+1]].fitness.nonlinearity - population[sorted_indices[k-1]].fitness.nonlinearity) / range;
        }
    }
}

fn tournament_select(population: &[Genome], tournament_size: usize) -> Genome {
    let mut rng = rand::thread_rng();
    let mut best_idx = rng.gen_range(0..population.len());
    for _ in 1..tournament_size {
        let idx = rng.gen_range(0..population.len());
        if population[idx].rank < population[best_idx].rank
            || (population[idx].rank == population[best_idx].rank && population[idx].crowding > population[best_idx].crowding)
        {
            best_idx = idx;
        }
    }
    population[best_idx].clone()
}

fn crossover(parent1: &Genome, parent2: &Genome) -> Genome {
    let mut rng = rand::thread_rng();
    let crossover_mask = rng.gen::<u8>();
    let enc_lut = (parent1.enc_lut & crossover_mask) | (parent2.enc_lut & !crossover_mask);

    let crossover_mask = rng.gen::<u8>();
    let eval_lut = (parent1.eval_lut & crossover_mask) | (parent2.eval_lut & !crossover_mask);

    let steps_avg = (parent1.steps + parent2.steps) / 2;
    let steps_noise = rng.gen_range(-2..=2);
    let steps = ((steps_avg as isize + steps_noise).max(4).min(128)) as usize;

    Genome {
        enc_lut,
        eval_lut,
        steps,
        grid_size: parent1.grid_size,
        fitness: Fitness::default(),
        rank: 0,
        crowding: 0.0,
    }
}

fn mutate(genome: &mut Genome, mutation_rate: f64) {
    let mut rng = rand::thread_rng();

    if rng.gen_bool(mutation_rate) {
        let bit = rng.gen_range(0..8);
        genome.enc_lut ^= 1 << bit;
    }

    if rng.gen_bool(mutation_rate) {
        let bit = rng.gen_range(0..8);
        genome.eval_lut ^= 1 << bit;
    }

    if rng.gen_bool(mutation_rate * 0.5) {
        let diff = rng.gen_range(-4..=4);
        genome.steps = ((genome.steps as isize + diff).max(4).min(128)) as usize;
    }

    if rng.gen_bool(mutation_rate * 0.3) {
        let bit1 = rng.gen_range(0..8);
        let bit2 = rng.gen_range(0..8);
        let v1 = (genome.enc_lut >> bit1) & 1;
        let v2 = (genome.enc_lut >> bit2) & 1;
        if v1 != v2 {
            genome.enc_lut ^= (1 << bit1) | (1 << bit2);
        }
    }
}

fn random_genome(grid_size: usize, steps_range: (usize, usize)) -> Genome {
    let mut rng = rand::thread_rng();
    let mut enc = rng.gen::<u8>();
    while is_linear_rule(enc) {
        enc = rng.gen::<u8>();
    }
    let eval_lut = rng.gen::<u8>();
    let steps = rng.gen_range(steps_range.0..=steps_range.1);

    Genome {
        enc_lut: enc,
        eval_lut,
        steps,
        grid_size,
        fitness: Fitness::default(),
        rank: 0,
        crowding: 0.0,
    }
}

fn generate_test_pairs(
    k_bits: usize,
    grid_size: usize,
    num_pairs: usize,
) -> Vec<(BitGrid, BitGrid, u64, u64)> {
    let mut rng = rand::thread_rng();
    let mut pairs = Vec::with_capacity(num_pairs);
    let mask = if k_bits == 64 { u64::MAX } else { (1u64 << k_bits) - 1 };

    for _ in 0..num_pairs {
        let val_a = rng.gen::<u64>() & mask;
        let val_b = rng.gen::<u64>() & mask;
        let grid_a = encode_repetition(val_a, k_bits, grid_size);
        let grid_b = encode_repetition(val_b, k_bits, grid_size);
        pairs.push((grid_a, grid_b, val_a, val_b));
    }
    pairs
}

fn run_search(
    grid_size: usize,
    k_bits: usize,
    population_size: usize,
    generations: usize,
    num_test_pairs: usize,
    crossover_rate: f64,
    mutation_rate: f64,
) -> Vec<Genome> {
    // Generate test pairs for both XOR and ADD
    let xor_pairs = generate_test_pairs(k_bits, grid_size, num_test_pairs);
    let add_pairs = generate_test_pairs(k_bits, grid_size, num_test_pairs);

    // Initialize population
    let mut population = Vec::with_capacity(population_size);
    for _ in 0..population_size {
        population.push(random_genome(grid_size, (8, 64)));
    }

    // Add some known interesting rule combinations (linear ones as anchor points)
    let linear_anchors = [90, 150, 60, 102];
    for &rule in &linear_anchors {
        population.push(Genome {
            enc_lut: rule,
            eval_lut: rule,
            steps: 16,
            grid_size,
            fitness: Fitness::default(),
            rank: 0,
            crowding: 0.0,
        });
    }
    population.truncate(population_size);

    println!("{}", "=".repeat(70));
    println!("RUST CA-HE EVOLUTIONARY SEARCH (NSGA-II)");
    println!("{}", "=".repeat(70));
    println!("Grid: {}, Plaintext: {}-bit, Pop: {}, Gens: {}", grid_size, k_bits, population_size, generations);
    println!("Crossover: {:.2}, Mutation: {:.2}, Test pairs: {}", crossover_rate, mutation_rate, num_test_pairs);
    println!();

    let start_time = Instant::now();
    let mut best_fitness = 0.0;
    let mut stagnation_counter = 0;

    for gen in 0..generations {
        // Evaluate fitness
        for ind in &mut population {
            ind.fitness = evaluate_fitness(ind.enc_lut, ind.eval_lut, ind.steps, grid_size, k_bits, &xor_pairs, &add_pairs);
        }

        // Pareto rank
        let fronts = non_dominated_sort(&mut population);

        // Crowding distance
        for front in &fronts {
            calculate_crowding_distance(&mut population, front);
        }

        // Track best
        let current_best = population.iter().max_by(|a, b| a.fitness.aggregate.partial_cmp(&b.fitness.aggregate).unwrap()).unwrap();
        if current_best.fitness.aggregate > best_fitness {
            best_fitness = current_best.fitness.aggregate;
            stagnation_counter = 0;
        } else {
            stagnation_counter += 1;
        }

        // Print progress
        if gen % 20 == 0 || gen == generations - 1 {
            let front0_size = if fronts.is_empty() { 0 } else { fronts[0].len() };
            let avg_xor = population.iter().map(|g| g.fitness.homo_xor).sum::<f64>() / population.len() as f64;
            let avg_add = population.iter().map(|g| g.fitness.homo_add).sum::<f64>() / population.len() as f64;
            let best_xor = population.iter().map(|g| g.fitness.homo_xor).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
            let best_add = population.iter().map(|g| g.fitness.homo_add).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
            let best_nl = population.iter().map(|g| g.fitness.nonlinearity).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
            let elapsed = start_time.elapsed().as_secs_f64();

            println!(
                "Gen {:4} | Front0: {:3} | Best XOR: {:.3} | Best ADD: {:.3} | Avg XOR: {:.3} | Avg ADD: {:.3} | Best NL: {:.2} | Best Agg: {:.3} | {:.1}s",
                gen, front0_size, best_xor, best_add, avg_xor, avg_add, best_nl, best_fitness, elapsed
            );
        }

        // Early convergence check: if we find a nonlinear rule pair with XOR accuracy == 1.0 and ADD accuracy == 1.0
        let mut converged = false;
        for g in &population {
            if g.fitness.homo_xor >= 0.99 && g.fitness.homo_add >= 0.99 && !is_linear_rule(g.enc_lut) {
                println!("\n* CONVERGENCE: Found perfect homomorphic NONLINEAR rule pair at gen {}!", gen);
                println!("  Enc LUT: {}, Eval LUT: {}, Steps: {}", g.enc_lut, g.eval_lut, g.steps);
                println!("  Homo XOR: {:.4}, Homo ADD: {:.4}", g.fitness.homo_xor, g.fitness.homo_add);
                converged = true;
                break;
            }
        }
        if converged {
            break;
        }

        // Stagnation handling: inject diversity
        if stagnation_counter > 100 {
            println!("  [!] Stagnation detected at gen {}, injecting diversity", gen);
            let n_inject = population_size / 4;
            for i in 0..n_inject {
                let idx = population_size - 1 - i;
                population[idx] = random_genome(grid_size, (8, 64));
                population[idx].fitness = evaluate_fitness(
                    population[idx].enc_lut,
                    population[idx].eval_lut,
                    population[idx].steps,
                    grid_size,
                    k_bits,
                    &xor_pairs,
                    &add_pairs,
                );
            }
            stagnation_counter = 0;
        }

        // Create offspring
        let mut offspring = Vec::with_capacity(population_size);
        let mut rng = rand::thread_rng();
        while offspring.len() < population_size {
            let mut child = if rng.gen_bool(crossover_rate) {
                let p1 = tournament_select(&population, 3);
                let p2 = tournament_select(&population, 3);
                crossover(&p1, &p2)
            } else {
                tournament_select(&population, 3)
            };
            mutate(&mut child, mutation_rate);
            offspring.push(child);
        }

        // Evaluate offspring
        for ind in &mut offspring {
            ind.fitness = evaluate_fitness(ind.enc_lut, ind.eval_lut, ind.steps, grid_size, k_bits, &xor_pairs, &add_pairs);
        }

        // Combine population and offspring
        let mut combined = population;
        combined.extend(offspring);

        // Sort combined
        let fronts = non_dominated_sort(&mut combined);
        for front in &fronts {
            calculate_crowding_distance(&mut combined, front);
        }

        // Keep best population_size
        combined.sort_by(|a, b| {
            a.rank.cmp(&b.rank).then_with(|| {
                b.crowding.partial_cmp(&a.crowding).unwrap()
            })
        });
        combined.truncate(population_size);
        population = combined;
    }

    println!("\nSearch complete in {:.2}s.", start_time.elapsed().as_secs_f64());
    population
}

fn main() {
    // We can search on N=64 with k=8 (8-bit plaintext, repetition factor 8x)
    let grid_size = 64;
    let k_bits = 8;
    let population_size = 100;
    let generations = 200;
    let num_test_pairs = 30;

    let final_pop = run_search(
        grid_size,
        k_bits,
        population_size,
        generations,
        num_test_pairs,
        0.7,
        0.15,
    );

    // Filter rank-0 individuals (Pareto front)
    let mut front0: Vec<Genome> = final_pop.into_iter().filter(|g| g.rank == 0).collect();
    // Sort by addition accuracy then xor accuracy
    front0.sort_by(|a, b| {
        b.fitness.homo_add.partial_cmp(&a.fitness.homo_add).unwrap().then_with(|| {
            b.fitness.homo_xor.partial_cmp(&a.fitness.homo_xor).unwrap()
        })
    });

    println!("\nPareto Front (Rank 0): {} individuals", front0.len());
    println!("{:<5} {:<5} {:<5} {:<10} {:<10} {:<10} {:<8} {:<8} {:<6}", 
             "Enc", "Eval", "Steps", "Homo_XOR", "Homo_ADD", "Avalanche", "NonLin", "Agg", "Linear");
    println!("{}", "-".repeat(80));

    for g in front0.iter().take(20) {
        let f = &g.fitness;
        let is_lin = if is_linear_rule(g.enc_lut) { "L" } else { "N" };
        println!(
            "{:<5} {:<5} {:<5} {:<10.4} {:<10.4} {:<10.4} {:<8.4} {:<8.4} {:<6}",
            g.enc_lut, g.eval_lut, g.steps, f.homo_xor, f.homo_add, f.avalanche, f.nonlinearity, f.aggregate, is_lin
        );
    }

    // Save final results to json
    let results_json = serde_json::json!({
        "grid_size": grid_size,
        "k_bits": k_bits,
        "pareto_front": front0.iter().map(|g| {
            serde_json::json!({
                "enc_lut": g.enc_lut,
                "eval_lut": g.eval_lut,
                "steps": g.steps,
                "fitness": {
                    "homo_xor": g.fitness.homo_xor,
                    "homo_add": g.fitness.homo_add,
                    "avalanche": g.fitness.avalanche,
                    "nonlinearity": g.fitness.nonlinearity,
                    "cost": g.fitness.cost,
                    "aggregate": g.fitness.aggregate,
                },
                "is_linear": is_linear_rule(g.enc_lut),
            })
        }).collect::<Vec<_>>(),
    });

    let results_dir = "../results";
    std::fs::create_dir_all(results_dir).unwrap();
    let file_path = format!("{}/evolutionary_search_rust_results.json", results_dir);
    let mut file = File::create(&file_path).unwrap();
    file.write_all(serde_json::to_string_pretty(&results_json).unwrap().as_bytes()).unwrap();
    println!("\nSaved results to {}", file_path);
}
