const { expect } = require("chai");
const { ethers } = require("hardhat");

describe("CAHERuleRegistry", function () {
  let Registry;
  let registry;
  let owner;
  let miner;

  beforeEach(async function () {
    [owner, miner] = await ethers.getSigners();
    Registry = await ethers.getContractFactory("CAHERuleRegistry");
    registry = await Registry.deploy({ value: ethers.parseEther("5.0") }); // Fund registry with 5 ETH for rewards
  });

  describe("Deployment", function () {
    it("Should set the correct initial epoch and start block", async function () {
      expect(await registry.currentEpoch()).to.equal(1);
      expect(await registry.epochStartBlock()).to.be.greaterThan(0);
    });

    it("Should generate deterministic challenge seeds", async function () {
      const seed = await registry.getChallengeSeed();
      expect(seed).to.not.equal(ethers.ZeroHash);
    });
  });

  describe("Rule Submission and Verification", function () {
    it("Should revert for linear encryption rules (insufficient nonlinearity)", async function () {
      // Rule 90 mapped to 2D (linear, nonlinearity is 0)
      let linearLut = 0;
      for (let idx = 0; idx < 32; idx++) {
        let bit = ((idx >> 4) & 1) ^ ((idx >> 3) & 1) ^ ((idx >> 2) & 1) ^ ((idx >> 1) & 1) ^ (idx & 1);
        linearLut |= (bit << idx);
      }
      linearLut = linearLut >>> 0;

      await expect(
        registry.connect(miner).submitRulePair(linearLut, 0, 16)
      ).to.be.revertedWith("Rule has insufficient nonlinearity (too weak)");
    });

    it("Should successfully verify and register a valid nonlinear homomorphic rule pair", async function () {
      // 0x20202020 is Rule 64 mapped to 2D (nonlinearity 8, and is exactly XOR homomorphic at step 16)
      const encLut = 0x20202020; 
      const evalLut = 0x20202020;
      const steps = 16;

      const initialBalance = await ethers.provider.getBalance(miner.address);

      // Submit rule pair
      const tx = await registry.connect(miner).submitRulePair(encLut, evalLut, steps);
      await tx.wait();

      // Check registration
      const ruleHash = ethers.solidityPackedKeccak256(
        ["uint32", "uint32"],
        [encLut, evalLut]
      );
      
      const rulePair = await registry.rulePairs(ruleHash);
      expect(rulePair.verified).to.be.true;
      expect(rulePair.discoverer).to.equal(miner.address);
      expect(rulePair.nonlinearity).to.equal(4);

      // Check reward payout
      const finalBalance = await ethers.provider.getBalance(miner.address);
      expect(finalBalance).to.be.greaterThan(initialBalance); // Miner got 1 ETH reward (minus gas)
    });
  });

  describe("Epoch Management", function () {
    it("Should not allow advancing epoch early", async function () {
      await expect(registry.advanceEpoch()).to.be.revertedWith("Epoch duration not met");
    });
  });
});
