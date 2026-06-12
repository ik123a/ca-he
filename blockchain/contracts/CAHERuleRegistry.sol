// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title CA-HE Rule Registry & Proof-of-Evolution Verifier
/// @notice Stores and cryptographically verifies homomorphic CA rule pairs
contract CAHERuleRegistry {

    struct RulePair {
        bytes32 ruleHash;          // keccak256(abi.encodePacked(encLut, evalLut))
        uint32  encLut;            // Encryption rule LUT
        uint32  evalLut;           // Evaluation rule LUT
        uint16  steps;             // Evolution steps
        uint256 nonlinearity;      // Evolved rule nonlinearity (0 to 12)
        address discoverer;        // Address of the miner
        uint256 epoch;             // Epoch of submission
        uint256 blockNumber;       // Submission block number
        bool    verified;          // Verification status
    }

    // State
    mapping(bytes32 => RulePair) public rulePairs;
    bytes32[] public ruleIndex;
    
    uint256 public currentEpoch;
    uint256 public epochStartBlock;
    uint256 public constant EPOCH_DURATION = 100;    // 100 blocks
    uint256 public constant REWARD_AMOUNT = 1 ether; // reward for verified rules

    // Thresholds
    uint256 public constant MIN_NONLINEARITY = 4;    // Minimum required nonlinearity distance (out of 12)
    uint256 public constant NUM_VERIFICATION_TRIALS = 3;

    event RulePairSubmitted(bytes32 indexed ruleHash, address indexed discoverer, uint256 epoch);
    event RulePairVerified(bytes32 indexed ruleHash, uint256 nonlinearity);
    event EpochAdvanced(uint256 indexed newEpoch, bytes32 challengeSeed);

    constructor() payable {
        currentEpoch = 1;
        epochStartBlock = block.number;
    }

    /// @notice Get challenge seed derived from epoch and block hash
    function getChallengeSeed() public view returns (bytes32) {
        return keccak256(abi.encodePacked(
            blockhash(epochStartBlock),
            currentEpoch
        ));
    }

    /// @notice Advance the epoch if duration has passed
    function advanceEpoch() external {
        require(block.number >= epochStartBlock + EPOCH_DURATION, "Epoch duration not met");
        currentEpoch++;
        epochStartBlock = block.number;
        emit EpochAdvanced(currentEpoch, getChallengeSeed());
    }

    /// @notice Submit and verify a candidate rule pair
    function submitRulePair(
        uint32 encLut,
        uint32 evalLut,
        uint16 steps
    ) external {
        bytes32 ruleHash = keccak256(abi.encodePacked(encLut, evalLut));
        require(rulePairs[ruleHash].blockNumber == 0, "Rule pair already submitted");
        require(steps >= 4 && steps <= 128, "Steps out of bounds");

        // 1. Verify Nonlinearity On-chain to enforce cryptographic security
        uint256 nl = nonlinearityScore2D(encLut);
        require(nl >= MIN_NONLINEARITY, "Rule has insufficient nonlinearity (too weak)");

        // 2. Generate deterministic test inputs from the challenge seed
        bytes32 seed = getChallengeSeed();
        (uint64[] memory testA, uint64[] memory testB) = generateTestInputs(seed, NUM_VERIFICATION_TRIALS);

        // 3. Cryptographically verify XOR homomorphism on-chain
        bool isHomomorphic = verifyXORHomomorphism(encLut, evalLut, steps, testA, testB);
        require(isHomomorphic, "Rule pair failed XOR homomorphism verification");

        // 4. Save and reward
        rulePairs[ruleHash] = RulePair({
            ruleHash: ruleHash,
            encLut: encLut,
            evalLut: evalLut,
            steps: steps,
            nonlinearity: nl,
            discoverer: msg.sender,
            epoch: currentEpoch,
            blockNumber: block.number,
            verified: true
        });

        ruleIndex.push(ruleHash);

        emit RulePairSubmitted(ruleHash, msg.sender, currentEpoch);
        emit RulePairVerified(ruleHash, nl);

        // Pay reward to the discoverer if contract has enough balance
        if (address(this).balance >= REWARD_AMOUNT) {
            payable(msg.sender).transfer(REWARD_AMOUNT);
        }
    }

    /// @notice Get total number of registered rule pairs
    function getRuleCount() external view returns (uint256) {
        return ruleIndex.length;
    }

    // ─────────────────────────────────────────────────────────────────────
    // 2D CA Simulation Helpers (Solidity Implementation)
    // ─────────────────────────────────────────────────────────────────────

    /// @dev Get cell (y, x) value from 8x8 packed grid
    function getCell(uint64 grid, uint256 y, uint256 x) public pure returns (uint64) {
        uint256 idx = ((y & 7) << 3) | (x & 7);
        return (grid >> idx) & 1;
    }

    /// @dev Set cell (y, x) value in 8x8 packed grid
    function setCell(uint64 grid, uint256 y, uint256 x, uint64 val) public pure returns (uint64) {
        uint256 idx = ((y & 7) << 3) | (x & 7);
        grid &= ~(uint64(1) << idx);
        grid |= (val & 1) << idx;
        return grid;
    }

    /// @dev Extract 5-input von Neumann neighborhood index for cell (y, x)
    function getNeighborhoodIdx(uint64 grid, uint256 y, uint256 x) public pure returns (uint256) {
        uint256 u = getCell(grid, (y + 7) & 7, x);
        uint256 d = getCell(grid, (y + 1) & 7, x);
        uint256 l = getCell(grid, y, (x + 7) & 7);
        uint256 r = getCell(grid, y, (x + 1) & 7);
        uint256 c = getCell(grid, y, x);
        return (u << 4) | (d << 3) | (l << 2) | (r << 1) | c;
    }

    /// @dev Apply a 2D CA step: next = apply_rule(curr) ^ prev (Optimized row-wise)
    function applyRule2DStep(uint64 prev, uint64 curr, uint32 ruleLut) public pure returns (uint64) {
        uint64 next = 0;
        uint256 r0 = uint8(curr);
        uint256 r1 = uint8(curr >> 8);
        uint256 r2 = uint8(curr >> 16);
        uint256 r3 = uint8(curr >> 24);
        uint256 r4 = uint8(curr >> 32);
        uint256 r5 = uint8(curr >> 40);
        uint256 r6 = uint8(curr >> 48);
        uint256 r7 = uint8(curr >> 56);

        for (uint256 y = 0; y < 8; y++) {
            uint256 u; // up row
            uint256 d; // down row
            uint256 c; // center row

            if (y == 0) { u = r7; c = r0; d = r1; }
            else if (y == 1) { u = r0; c = r1; d = r2; }
            else if (y == 2) { u = r1; c = r2; d = r3; }
            else if (y == 3) { u = r2; c = r3; d = r4; }
            else if (y == 4) { u = r3; c = r4; d = r5; }
            else if (y == 5) { u = r4; c = r5; d = r6; }
            else if (y == 6) { u = r5; c = r6; d = r7; }
            else { u = r6; c = r7; d = r0; }

            uint256 l = ((c << 1) | (c >> 7)) & 255;
            uint256 r = ((c >> 1) | (c << 7)) & 255;

            uint256 prev_row = uint8(prev >> (y << 3));
            uint256 next_row = 0;

            for (uint256 x = 0; x < 8; x++) {
                uint256 bit_u = (u >> x) & 1;
                uint256 bit_d = (d >> x) & 1;
                uint256 bit_l = (l >> x) & 1;
                uint256 bit_r = (r >> x) & 1;
                uint256 bit_c = (c >> x) & 1;

                uint256 idx = (bit_u << 4) | (bit_d << 3) | (bit_l << 2) | (bit_r << 1) | bit_c;
                uint256 bit_out = (ruleLut >> idx) & 1;
                next_row |= ((bit_out ^ ((prev_row >> x) & 1)) << x);
            }
            next |= (uint64(next_row) << (y << 3));
        }
        return next;
    }

    /// @dev Evolve the grid for a number of steps
    function evolve2D(uint64 prev, uint64 curr, uint32 ruleLut, uint256 steps) public pure returns (uint64, uint64) {
        uint64 p = prev;
        uint64 c = curr;
        for (uint256 i = 0; i < steps; i++) {
            uint64 next = applyRule2DStep(p, c, ruleLut);
            p = c;
            c = next;
        }
        return (p, c);
    }

    /// @dev Encrypt a 2D plaintext grid
    function encrypt2D(uint64 plaintext, uint64 iv, uint32 ruleLut, uint256 steps) public pure returns (uint64, uint64) {
        uint64 initial_prev = plaintext ^ iv;
        uint64 initial_curr = iv;
        return evolve2D(initial_prev, initial_curr, ruleLut, steps);
    }

    /// @dev Decrypt a 2D ciphertext grid pair
    function decrypt2D(uint64 c0, uint64 c1, uint64 iv, uint32 ruleLut, uint256 steps) public pure returns (uint64) {
        (, uint64 c_final) = evolve2D(c1, c0, ruleLut, steps);
        return c_final ^ iv;
    }

    /// @dev Check if a rule pair satisfies XOR homomorphism on given inputs
    function verifyXORHomomorphism(
        uint32 encLut,
        uint32 evalLut,
        uint16 steps,
        uint64[] memory testA,
        uint64[] memory testB
    ) public pure returns (bool) {
        uint64 iv = 0; // zero IV for simple verification
        for (uint256 i = 0; i < testA.length; i++) {
            uint64 a = testA[i];
            uint64 b = testB[i];

            (uint64 c_a0, uint64 c_a1) = encrypt2D(a, iv, encLut, steps);
            (uint64 c_b0, uint64 c_b1) = encrypt2D(b, iv, encLut, steps);

            uint64 c_sum0 = c_a0 ^ c_b0;
            uint64 c_sum1 = c_a1 ^ c_b1;

            (uint64 c_eval0, uint64 c_eval1) = evolve2D(c_sum0, c_sum1, evalLut, steps);

            uint64 dec = decrypt2D(c_eval0, c_eval1, iv, encLut, steps);
            uint64 expected = a ^ b;

            if (dec != expected) {
                return false;
            }
        }
        return true;
    }

    /// @dev Compute rule nonlinearity (0 to 12, higher is more secure)
    function nonlinearityScore2D(uint32 ruleLut) public pure returns (uint256) {
        uint256 minDist = 32;
        for (uint256 coef = 0; coef < 64; coef++) {
            uint256 dist = 0;
            for (uint256 idx = 0; idx < 32; idx++) {
                uint256 x0 = (idx >> 0) & 1;
                uint256 x1 = (idx >> 1) & 1;
                uint256 x2 = (idx >> 2) & 1;
                uint256 x3 = (idx >> 3) & 1;
                uint256 x4 = (idx >> 4) & 1;
                
                uint256 a0 = coef & 1;
                uint256 a1 = (coef >> 1) & 1;
                uint256 a2 = (coef >> 2) & 1;
                uint256 a3 = (coef >> 3) & 1;
                uint256 a4 = (coef >> 4) & 1;
                uint256 a5 = (coef >> 5) & 1;

                uint256 affine = (a5 * x4) ^ (a4 * x3) ^ (a3 * x2) ^ (a2 * x1) ^ (a1 * x0) ^ a0;
                uint256 actual = (ruleLut >> idx) & 1;
                if (affine != actual) {
                    dist++;
                }
            }
            if (dist < minDist) {
                minDist = dist;
            }
        }
        return minDist;
    }

    /// @dev Generate deterministic test inputs from seed
    function generateTestInputs(
        bytes32 seed,
        uint256 count
    ) public pure returns (uint64[] memory, uint64[] memory) {
        uint64[] memory testA = new uint64[](count);
        uint64[] memory testB = new uint64[](count);
        bytes32 temp = seed;
        for (uint256 i = 0; i < count; i++) {
            temp = keccak256(abi.encodePacked(temp, i));
            testA[i] = uint64(uint256(temp));
            temp = keccak256(abi.encodePacked(temp, i + 100));
            testB[i] = uint64(uint256(temp));
        }
        return (testA, testB);
    }

    receive() external payable {}
}
