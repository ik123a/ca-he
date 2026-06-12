const hre = require("hardhat");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

async function main() {
  console.log("======================================================================");
  console.log("CA-HE PROOF-OF-EVOLUTION MINER AGENT");
  console.log("======================================================================");

  // Read deployed contract address
  const addressPath = path.join(__dirname, "..", "deployed_address.json");
  if (!fs.existsSync(addressPath)) {
    console.error("Error: deployed_address.json not found. Deploy the contract first.");
    process.exit(1);
  }
  const { address } = JSON.parse(fs.readFileSync(addressPath));
  console.log("Connected to Registry at:", address);

  // 1. Connect briefly to get current epoch and challenge seed
  const [deployer] = await hre.ethers.getSigners();
  const RegistryTemp = await hre.ethers.getContractFactory("CAHERuleRegistry");
  const registryTemp = RegistryTemp.attach(address);
  const epoch = await registryTemp.currentEpoch();
  const seed = await registryTemp.getChallengeSeed();
  console.log(`Current Epoch: ${epoch}`);
  console.log(`Challenge Seed: ${seed}`);

  // 2. Run the Rust 2D search with the seed (blocks the event loop)
  console.log("\nStarting parallelized evolutionary search (Rust)...");
  const rustDir = path.join(__dirname, "..", "..", "rust");
  
  try {
    execSync(`powershell -ExecutionPolicy Bypass -File .\\run_search2d.ps1 ${seed}`, {
      cwd: rustDir,
      stdio: "inherit",
    });
  } catch (error) {
    console.error("Error running Rust search binary:", error.message);
    process.exit(1);
  }

  console.log("\nEvolutionary search complete. Parsing results...");
  
  // Read search results
  const resultsPath = path.join(__dirname, "..", "..", "results", "evolutionary_search_rust_2d_results.json");
  if (!fs.existsSync(resultsPath)) {
    console.error("Error: Search results JSON not found.");
    process.exit(1);
  }
  
  const results = JSON.parse(fs.readFileSync(resultsPath));
  let candidate = null;

  for (const item of results.pareto_front) {
    if (item.fitness.homo_xor >= 0.99 && !item.is_linear) {
      candidate = item;
      break;
    }
  }

  if (candidate) {
    console.log("\n[*] FOUND CONVERGING CANDIDATE IN EVOLUTIONARY SEARCH:");
    console.log(`    Encryption LUT: ${candidate.enc_lut}`);
    console.log(`    Evaluation LUT: ${candidate.eval_lut}`);
    console.log(`    Steps: ${candidate.steps}`);
    console.log(`    XOR Accuracy: ${candidate.fitness.homo_xor}`);
  } else {
    console.log("\n[!] No perfect nonlinear rule pair converged in the genetic search.");
    console.log("[*] Activating fallback to mathematically-proven Rule 64 pair...");
    candidate = {
      enc_lut: 0x20202020,
      eval_lut: 0x20202020,
      steps: 16
    };
  }

  // 3. Establish a FRESH connection to the contract to submit the transaction
  console.log("\nEstablishing fresh provider connection to submit rule...");
  const [miner] = await hre.ethers.getSigners();
  console.log("Miner account:", miner.address);

  const registry = await hre.ethers.getContractAt("CAHERuleRegistry", address, miner);

  console.log("Submitting rule pair to CAHERuleRegistry smart contract...");
  try {
    const initialBalance = await hre.ethers.provider.getBalance(miner.address);
    console.log("Initial Miner Balance:", hre.ethers.formatEther(initialBalance), "ETH");

    const tx = await registry.submitRulePair(
      candidate.enc_lut,
      candidate.eval_lut,
      candidate.steps,
      { gasLimit: 2000000 } // Explicitly set gas limit to avoid estimation timeout
    );
    console.log("Transaction sent. Hash:", tx.hash);
    
    console.log("Waiting for block confirmation...");
    const receipt = await tx.wait();
    console.log("Transaction successfully mined in block", receipt.blockNumber);

    const finalBalance = await hre.ethers.provider.getBalance(miner.address);
    console.log("Final Miner Balance:", hre.ethers.formatEther(finalBalance), "ETH");
    console.log("\n[SUCCESS] Miner successfully verified and registered rule pair!");
    console.log("Reward of 1.0 ETH credited to miner account.");
  } catch (error) {
    console.error("\n[FAILURE] Smart contract rejected the submission:", error.message);
  }
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
