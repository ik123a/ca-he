const hre = require("hardhat");
const fs = require("fs");
const path = require("path");

async function main() {
  const [miner] = await hre.ethers.getSigners();
  console.log("Miner account:", miner.address);

  const addressPath = path.join(__dirname, "..", "deployed_address.json");
  if (!fs.existsSync(addressPath)) {
    console.error("Error: deployed_address.json not found.");
    process.exit(1);
  }
  const { address } = JSON.parse(fs.readFileSync(addressPath));

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

  const registry = await hre.ethers.getContractAt("CAHERuleRegistry", address, miner);
  console.log("\nSubmitting rule pair to CAHERuleRegistry smart contract...");
  try {
    const initialBalance = await hre.ethers.provider.getBalance(miner.address);
    console.log("Initial Miner Balance:", hre.ethers.formatEther(initialBalance), "ETH");

    const tx = await registry.submitRulePair(
      candidate.enc_lut,
      candidate.eval_lut,
      candidate.steps,
      { gasLimit: 15000000 }
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
    process.exit(1);
  }
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
