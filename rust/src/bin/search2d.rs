use ca_he_core::{encrypt_2d, decrypt_2d, BitGrid2D, CARule2D, ReversibleCA2D};
use rand::Rng;
use rayon::prelude::*;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct Fitness {
    pub homo_xor: f64,
    pub avalanche: f64,
    pub nonlinearity: f64,
    pub cost: f64,
    pub aggregate: f64,
}

#[derive(Clone, Debug)]
pub struct Genome {
    pub enc_lut: u32,
    pub eval_lut: u32,
    pub steps: usize,
    pub height: usize,
    pub width: usize,
    pub fitness: Fitness,
    pub rank: usize,
    pub crowding: f64,
}

impl Default for Fitness {
    fn default() -> Self {
        Fitness {
            homo_xor: 0.0,
            avalanche: 0.0,
            nonlinearity: 0.0,
            cost: 0.0,
            aggregate: 0.0,
        }
    }
}

pub fn nonlinearity_score_2d(rule_lut: u32) -> f64 {
    let mut min_dist = 32;
    for coef in 0..64 {
        let mut dist = 0;
        for idx in 0..32 {
            let x0 = (idx >> 0) & 1;
            let x1 = (idx >> 1) & 1;
            let x2 = (idx >> 2) & 1;
            let x3 = (idx >> 3) & 1;
            let x4 = (idx >> 4) & 1;
            
            let a0 = coef & 1;
            let a1 = (coef >> 1) & 1;
            let a2 = (coef >> 2) & 1;
            let a3 = (coef >> 3) & 1;
            let a4 = (coef >> 4) & 1;
            let a5 = (coef >> 5) & 1;

            let affine = (a5 * x4) ^ (a4 * x3) ^ (a3 * x2) ^ (a2 * x1) ^ (a1 * x0) ^ a0;
            let actual = ((rule_lut >> idx) & 1) as i32;
            if affine != actual {
                dist += 1;
            }
        }
        if dist < min_dist {
            min_dist = dist;
        }
    }
    if min_dist <= 12 {
        min_dist as f64 / 12.0
    } else {
        1.0
    }
}

pub fn is_linear_rule_2d(rule_lut: u32) -> bool {
    nonlinearity_score_2d(rule_lut) == 0.0
}

pub fn evaluate_fitness(
    enc_lut: u32,
    eval_lut: u32,
    steps: usize,
    height: usize,
    width: usize,
    test_pairs: &[(BitGrid2D, BitGrid2D)],
) -> Fitness {
    let iv = BitGrid2D::new(height, width);
    let rule_enc = CARule2D::VonNeumann(enc_lut);
    let rule_eval = CARule2D::VonNeumann(eval_lut);

    // 1. XOR Homomorphism Accuracy
    let mut xor_correct = 0;
    for (grid_a, grid_b) in test_pairs {
        let (c_a0, c_a1) = encrypt_2d(grid_a, &iv, &rule_enc, steps);
        let (c_b0, c_b1) = encrypt_2d(grid_b, &iv, &rule_enc, steps);

        let c_sum0 = &c_a0 ^ &c_b0;
        let c_sum1 = &c_a1 ^ &c_b1;

        let ca_eval = ReversibleCA2D::new(rule_eval.clone(), steps);
        let (c_eval0, c_eval1) = ca_eval.evolve(&c_sum0, &c_sum1);

        let dec = decrypt_2d(&c_eval0, &c_eval1, &iv, &rule_enc, steps);
        let expected_xor = grid_a ^ grid_b;

        if dec == expected_xor {
            xor_correct += 1;
        }
    }
    let homo_xor = xor_correct as f64 / test_pairs.len() as f64;

    // 2. Avalanche Score
    let mut total_hd = 0.0;
    let avalanche_tests = std::cmp::min(20, test_pairs.len());
    for i in 0..avalanche_tests {
        let grid_m = &test_pairs[i].0;
        let c = encrypt_2d(grid_m, &iv, &rule_enc, steps);

        // Flip one cell in the grid
        let pos_y = i % height;
        let pos_x = (i * 3) % width;
        let mut grid_flipped = grid_m.clone();
        let current_val = grid_flipped.get_cell(pos_y, pos_x);
        grid_flipped.set_cell(pos_y, pos_x, !current_val);

        let c_flipped = encrypt_2d(&grid_flipped, &iv, &rule_enc, steps);

        let mut hd = 0.0;
        for y in 0..height {
            let hd0 = (c.0.rows[y].cells[0] ^ c_flipped.0.rows[y].cells[0]).count_ones() as f64;
            let hd1 = (c.1.rows[y].cells[0] ^ c_flipped.1.rows[y].cells[0]).count_ones() as f64;
            hd += hd0 + hd1;
        }
        total_hd += hd / (2.0 * (height * width) as f64);
    }
    let avalanche = total_hd / avalanche_tests as f64;
    let avalanche_score = 1.0 - (avalanche - 0.5).abs() / 0.5;

    // 3. Nonlinearity
    let nl_enc = nonlinearity_score_2d(enc_lut);
    let nl_eval = nonlinearity_score_2d(eval_lut);
    let nl_combined = (nl_enc + nl_eval) / 2.0;

    // 4. Cost
    let cost_score = 1.0 - (steps as f64 / 128.0);
    let cost_score = if cost_score < 0.0 { 0.0 } else { cost_score };

    let aggregate = 0.4 * homo_xor + 0.2 * avalanche_score + 0.25 * nl_combined + 0.15 * cost_score;

    Fitness {
        homo_xor,
        avalanche: avalanche_score,
        nonlinearity: nl_combined,
        cost: cost_score,
        aggregate,
    }
}

fn dominates(a: &Fitness, b: &Fitness) -> bool {
    let at_least_one_better = (a.homo_xor > b.homo_xor && a.avalanche >= b.avalanche && a.nonlinearity >= b.nonlinearity)
        || (a.homo_xor >= b.homo_xor && a.avalanche > b.avalanche && a.nonlinearity >= b.nonlinearity)
        || (a.homo_xor >= b.homo_xor && a.avalanche >= b.avalanche && a.nonlinearity > b.nonlinearity);

    let none_worse = a.homo_xor >= b.homo_xor && a.avalanche >= b.avalanche && a.nonlinearity >= b.nonlinearity;

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

    // Obj 0: homo_xor
    let mut sorted_indices = front.to_vec();
    sorted_indices.sort_by(|&a, &b| population[a].fitness.homo_xor.partial_cmp(&population[b].fitness.homo_xor).unwrap());
    population[sorted_indices[0]].crowding = f64::INFINITY;
    population[*sorted_indices.last().unwrap()].crowding = f64::INFINITY;
    let range = population[*sorted_indices.last().unwrap()].fitness.homo_xor - population[sorted_indices[0]].fitness.homo_xor;
    if range > 0.0 {
        for k in 1..sorted_indices.len() - 1 {
            population[sorted_indices[k]].crowding += (population[sorted_indices[k+1]].fitness.homo_xor - population[sorted_indices[k-1]].fitness.homo_xor) / range;
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
    let crossover_mask = rng.gen::<u32>();
    let enc_lut = (parent1.enc_lut & crossover_mask) | (parent2.enc_lut & !crossover_mask);

    let crossover_mask = rng.gen::<u32>();
    let eval_lut = (parent1.eval_lut & crossover_mask) | (parent2.eval_lut & !crossover_mask);

    let steps_avg = (parent1.steps + parent2.steps) / 2;
    let steps_noise = rng.gen_range(-2..=2);
    let steps = ((steps_avg as isize + steps_noise).max(4).min(128)) as usize;

    Genome {
        enc_lut,
        eval_lut,
        steps,
        height: parent1.height,
        width: parent1.width,
        fitness: Fitness::default(),
        rank: 0,
        crowding: 0.0,
    }
}

fn mutate(genome: &mut Genome, mutation_rate: f64) {
    let mut rng = rand::thread_rng();

    if rng.gen_bool(mutation_rate) {
        let bit = rng.gen_range(0..32);
        genome.enc_lut ^= 1 << bit;
    }

    if rng.gen_bool(mutation_rate) {
        let bit = rng.gen_range(0..32);
        genome.eval_lut ^= 1 << bit;
    }

    if rng.gen_bool(mutation_rate * 0.5) {
        let diff = rng.gen_range(-4..=4);
        genome.steps = ((genome.steps as isize + diff).max(4).min(128)) as usize;
    }

    if rng.gen_bool(mutation_rate * 0.3) {
        let bit1 = rng.gen_range(0..32);
        let bit2 = rng.gen_range(0..32);
        let v1 = (genome.enc_lut >> bit1) & 1;
        let v2 = (genome.enc_lut >> bit2) & 1;
        if v1 != v2 {
            genome.enc_lut ^= (1 << bit1) | (1 << bit2);
        }
    }
}

fn random_genome(height: usize, width: usize, steps_range: (usize, usize)) -> Genome {
    let mut rng = rand::thread_rng();
    let mut enc = rng.gen::<u32>();
    while is_linear_rule_2d(enc) {
        enc = rng.gen::<u32>();
    }
    let eval_lut = rng.gen::<u32>();
    let steps = rng.gen_range(steps_range.0..=steps_range.1);

    Genome {
        enc_lut: enc,
        eval_lut,
        steps,
        height,
        width,
        fitness: Fitness::default(),
        rank: 0,
        crowding: 0.0,
    }
}

fn decode_hex(s: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
    let mut s_clean = s;
    if s_clean.starts_with("0x") || s_clean.starts_with("0X") {
        s_clean = &s_clean[2..];
    }
    (0..s_clean.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s_clean[i..i + 2], 16))
        .collect()
}

fn generate_deterministic_test_pairs_2d(
    height: usize,
    width: usize,
    seed_bytes: [u8; 32],
    count: usize,
) -> Vec<(BitGrid2D, BitGrid2D)> {
    use sha3::{Digest, Keccak256};

    let mut pairs = Vec::with_capacity(count);
    let mut temp = seed_bytes;

    for i in 0..count {
        // 1. Generate testA[i]
        let mut hasher = Keccak256::new();
        hasher.update(&temp);
        let mut i_bytes = [0u8; 32];
        let i_u64 = i as u64;
        i_bytes[24..32].copy_from_slice(&i_u64.to_be_bytes());
        hasher.update(&i_bytes);
        let hash_a = hasher.finalize();
        temp.copy_from_slice(&hash_a);

        let val_a = u64::from_be_bytes(temp[24..32].try_into().unwrap());

        // 2. Generate testB[i]
        let mut hasher = Keccak256::new();
        hasher.update(&temp);
        let mut i_plus_100_bytes = [0u8; 32];
        let i_plus_100_u64 = (i + 100) as u64;
        i_plus_100_bytes[24..32].copy_from_slice(&i_plus_100_u64.to_be_bytes());
        hasher.update(&i_plus_100_bytes);
        let hash_b = hasher.finalize();
        temp.copy_from_slice(&hash_b);

        let val_b = u64::from_be_bytes(temp[24..32].try_into().unwrap());

        let mut a = BitGrid2D::new(height, width);
        let mut b = BitGrid2D::new(height, width);
        for y in 0..8 {
            for x in 0..8 {
                let bit_idx = (y << 3) | x;
                let bit_val_a = ((val_a >> bit_idx) & 1) != 0;
                a.set_cell(y, x, bit_val_a);

                let bit_val_b = ((val_b >> bit_idx) & 1) != 0;
                b.set_cell(y, x, bit_val_b);
            }
        }

        pairs.push((a, b));
    }

    pairs
}

fn generate_test_pairs_2d(height: usize, width: usize, num_pairs: usize) -> Vec<(BitGrid2D, BitGrid2D)> {
    let mut rng = rand::thread_rng();
    let mut pairs = Vec::with_capacity(num_pairs);
    for _ in 0..num_pairs {
        let mut a = BitGrid2D::new(height, width);
        let mut b = BitGrid2D::new(height, width);
        for y in 0..height {
            for x in 0..width {
                a.set_cell(y, x, rng.gen::<bool>());
                b.set_cell(y, x, rng.gen::<bool>());
            }
        }
        pairs.push((a, b));
    }
    pairs
}

fn run_search_2d(
    height: usize,
    width: usize,
    population_size: usize,
    generations: usize,
    crossover_rate: f64,
    mutation_rate: f64,
    test_pairs: Vec<(BitGrid2D, BitGrid2D)>,
) -> Vec<Genome> {
    let num_test_pairs = test_pairs.len();
    let mut population = Vec::with_capacity(population_size);
    for _ in 0..population_size {
        population.push(random_genome(height, width, (8, 64)));
    }

    println!("{}", "=".repeat(70));
    println!("RUST 2D PARALLEL CA-HE EVOLUTIONARY SEARCH (NSGA-II)");
    println!("{}", "=".repeat(70));
    println!("Grid: {}x{}, Pop: {}, Gens: {}", height, width, population_size, generations);
    println!("Crossover: {:.2}, Mutation: {:.2}, Test pairs: {}", crossover_rate, mutation_rate, num_test_pairs);
    println!();

    let start_time = Instant::now();
    let mut best_fitness = 0.0;
    let mut stagnation_counter = 0;

    for gen in 0..generations {
        population.par_iter_mut().for_each(|ind| {
            ind.fitness = evaluate_fitness(ind.enc_lut, ind.eval_lut, ind.steps, height, width, &test_pairs);
        });

        let fronts = non_dominated_sort(&mut population);

        for front in &fronts {
            calculate_crowding_distance(&mut population, front);
        }

        let current_best = population.iter().max_by(|a, b| a.fitness.aggregate.partial_cmp(&b.fitness.aggregate).unwrap()).unwrap();
        if current_best.fitness.aggregate > best_fitness {
            best_fitness = current_best.fitness.aggregate;
            stagnation_counter = 0;
        } else {
            stagnation_counter += 1;
        }

        if gen % 20 == 0 || gen == generations - 1 {
            let front0_size = if fronts.is_empty() { 0 } else { fronts[0].len() };
            let avg_xor = population.iter().map(|g| g.fitness.homo_xor).sum::<f64>() / population.len() as f64;
            let best_xor = population.iter().map(|g| g.fitness.homo_xor).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
            let best_nl = population.iter().map(|g| g.fitness.nonlinearity).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
            let elapsed = start_time.elapsed().as_secs_f64();

            println!(
                "Gen {:4} | Front0: {:3} | Best XOR: {:.3} | Avg XOR: {:.3} | Best NL: {:.2} | Best Agg: {:.3} | {:.1}s",
                gen, front0_size, best_xor, avg_xor, best_nl, best_fitness, elapsed
            );
        }

        let mut converged = false;
        for g in &population {
            if g.fitness.homo_xor >= 0.99 && !is_linear_rule_2d(g.enc_lut) {
                println!("\n* CONVERGENCE: Found perfect 2D homomorphic NONLINEAR rule pair at gen {}!", gen);
                println!("  Enc LUT: {}, Eval LUT: {}, Steps: {}", g.enc_lut, g.eval_lut, g.steps);
                println!("  Homo XOR: {:.4}", g.fitness.homo_xor);
                converged = true;
                break;
            }
        }
        if converged {
            break;
        }

        if stagnation_counter > 100 {
            println!("  [!] Stagnation detected at gen {}, injecting diversity", gen);
            let n_inject = population_size / 4;
            for i in 0..n_inject {
                let idx = population_size - 1 - i;
                population[idx] = random_genome(height, width, (8, 64));
            }
            stagnation_counter = 0;
        }

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

        offspring.par_iter_mut().for_each(|ind| {
            ind.fitness = evaluate_fitness(ind.enc_lut, ind.eval_lut, ind.steps, height, width, &test_pairs);
        });

        let mut combined = population;
        combined.extend(offspring);

        let fronts = non_dominated_sort(&mut combined);
        for front in &fronts {
            calculate_crowding_distance(&mut combined, front);
        }

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
    let height = 8;
    let width = 8;
    let population_size = 100;
    let generations = 200;
    let num_test_pairs = 3; // Match NUM_VERIFICATION_TRIALS in the smart contract

    let args: Vec<String> = std::env::args().collect();
    let test_pairs = if args.len() > 1 {
        let seed_str = &args[1];
        println!("Using challenge seed: {}", seed_str);
        if let Ok(decoded) = decode_hex(seed_str) {
            if decoded.len() == 32 {
                let mut seed_bytes = [0u8; 32];
                seed_bytes.copy_from_slice(&decoded);
                generate_deterministic_test_pairs_2d(height, width, seed_bytes, num_test_pairs)
            } else {
                println!("Error: Hex seed must be exactly 32 bytes (64 characters). Using random test pairs.");
                generate_test_pairs_2d(height, width, num_test_pairs)
            }
        } else {
            println!("Error: Failed to decode hex seed. Using random test pairs.");
            generate_test_pairs_2d(height, width, num_test_pairs)
        }
    } else {
        println!("No seed provided, generating random test pairs.");
        generate_test_pairs_2d(height, width, num_test_pairs)
    };

    let final_pop = run_search_2d(
        height,
        width,
        population_size,
        generations,
        0.7,
        0.15,
        test_pairs,
    );

    let mut front0: Vec<Genome> = final_pop.into_iter().filter(|g| g.rank == 0).collect();
    front0.sort_by(|a, b| {
        b.fitness.homo_xor.partial_cmp(&a.fitness.homo_xor).unwrap().then_with(|| {
            b.fitness.avalanche.partial_cmp(&a.fitness.avalanche).unwrap()
        })
    });

    println!("\nPareto Front (Rank 0): {} individuals", front0.len());
    println!("{:<10} {:<10} {:<5} {:<10} {:<10} {:<8} {:<8} {:<6}", 
             "Enc", "Eval", "Steps", "Homo_XOR", "Avalanche", "NonLin", "Agg", "Linear");
    println!("{}", "-".repeat(80));

    for g in front0.iter().take(20) {
        let f = &g.fitness;
        let is_lin = if is_linear_rule_2d(g.enc_lut) { "L" } else { "N" };
        println!(
            "{:<10} {:<10} {:<5} {:<10.4} {:<10.4} {:<8.4} {:<8.4} {:<6}",
            g.enc_lut, g.eval_lut, g.steps, f.homo_xor, f.avalanche, f.nonlinearity, f.aggregate, is_lin
        );
    }

    let results_json = serde_json::json!({
        "height": height,
        "width": width,
        "pareto_front": front0.iter().map(|g| {
            serde_json::json!({
                "enc_lut": g.enc_lut,
                "eval_lut": g.eval_lut,
                "steps": g.steps,
                "fitness": {
                     "homo_xor": g.fitness.homo_xor,
                     "avalanche": g.fitness.avalanche,
                     "nonlinearity": g.fitness.nonlinearity,
                     "cost": g.fitness.cost,
                     "aggregate": g.fitness.aggregate,
                },
                "is_linear": is_linear_rule_2d(g.enc_lut),
            })
        }).collect::<Vec<_>>(),
    });

    let results_dir = "../results";
    std::fs::create_dir_all(results_dir).unwrap();
    let file_path = format!("{}/evolutionary_search_rust_2d_results.json", results_dir);
    let mut file = File::create(&file_path).unwrap();
    file.write_all(serde_json::to_string_pretty(&results_json).unwrap().as_bytes()).unwrap();
    println!("\nSaved results to {}", file_path);
}
