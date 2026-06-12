const hre = require("hardhat");
const fs = require("fs");
const path = require("path");

async function main() {
  const [deployer] = await hre.ethers.getSigners();
  console.log("Deploying contract with the account:", deployer.address);

  // Deploy CAHERuleRegistry funding it with 5 ETH for miner rewards
  const Registry = await hre.ethers.getContractFactory("CAHERuleRegistry");
  const registry = await Registry.deploy({ value: hre.ethers.parseEther("5.0") });
  await registry.waitForDeployment();

  const address = await registry.getAddress();
  console.log("CAHERuleRegistry deployed to:", address);

  // Save the address locally
  const addressPath = path.join(__dirname, "..", "deployed_address.json");
  fs.writeFileSync(
    addressPath,
    JSON.stringify({ address: address }, null, 2)
  );
  console.log("Saved deployed address to:", addressPath);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
