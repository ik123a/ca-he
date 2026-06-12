const hre = require("hardhat");
const fs = require("fs");
const path = require("path");

async function main() {
  const addressPath = path.join(__dirname, "..", "deployed_address.json");
  if (!fs.existsSync(addressPath)) {
    console.error("Error: deployed_address.json not found.");
    process.exit(1);
  }
  const { address } = JSON.parse(fs.readFileSync(addressPath));
  
  const Registry = await hre.ethers.getContractFactory("CAHERuleRegistry");
  const registry = Registry.attach(address);
  const seed = await registry.getChallengeSeed();
  
  // Output seed to stdout so it can be captured by calling process
  console.log(seed);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
