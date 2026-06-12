// CA-HE Reversible CA Dashboard & Interactive Sandbox Simulator
// v1.2 Core Logic Engine

document.addEventListener("DOMContentLoaded", () => {
    // State Variables
    let currentDimension = "1D"; // "1D" or "2D"
    let seedValue = "11390111231919993";
    let keySteps = 50;
    let radius = 1;
    let ruleset = "43-36";
    let iterations = 5;
    let updateRuleActive = true;
    let timelineSliderVal = 50;

    // Grab UI Elements
    const rulesetSelect = document.getElementById("ruleset-select");
    const radiusSlider = document.getElementById("radius-slider");
    const radiusVal = document.getElementById("radius-val");
    const dimToggle = document.getElementById("dim-toggle");
    const keyGenSteps = document.getElementById("key-gen-steps");
    const inetBtn = document.getElementById("inet-btn");
    const strengthSlider = document.getElementById("strength-slider");
    const strengthBars = document.getElementById("strength-bars");
    const seedInput = document.getElementById("seed-input");
    const iterationsSpinner = document.getElementById("iterations-spinner");
    const updateRuleToggle = document.getElementById("update-rule-toggle");
    const encryptionTimelineSlider = document.getElementById("encryption-timeline-slider");
    const timelineProgress = document.getElementById("timeline-progress");

    // Sandbox UI Elements
    const inputA = document.getElementById("input-a");
    const inputB = document.getElementById("input-b");
    const calcOp = document.getElementById("calc-op");
    const runCalcBtn = document.getElementById("run-calc-btn");
    const calcResultVal = document.getElementById("calc-result-val");
    const sandboxBadge = document.getElementById("sandbox-badge");

    // Initialize UI Text
    dimToggle.innerText = "1D Dimension";
    dimToggle.classList.add("active");

    // ─────────────────────────────────────────────────────────────────
    // Event Handlers for UI Controls
    // ─────────────────────────────────────────────────────────────────

    radiusSlider.addEventListener("input", (e) => {
        radius = parseInt(e.target.value);
        radiusVal.innerText = radius;
        runSimulation();
    });

    rulesetSelect.addEventListener("change", (e) => {
        ruleset = e.target.value;
        runSimulation();
    });

    dimToggle.addEventListener("click", () => {
        if (currentDimension === "1D") {
            currentDimension = "2D";
            dimToggle.innerText = "2D Dimension";
            dimToggle.style.background = "linear-gradient(90deg, #2979ff, #651fff)";
            dimToggle.style.boxShadow = "0 4px 15px rgba(41, 121, 255, 0.3)";
        } else {
            currentDimension = "1D";
            dimToggle.innerText = "1D Dimension";
            dimToggle.style.background = "linear-gradient(90deg, #9b51e0, #e040fb)";
            dimToggle.style.boxShadow = "0 4px 15px rgba(224, 64, 251, 0.3)";
        }
        runSimulation();
    });

    keyGenSteps.addEventListener("input", (e) => {
        keySteps = parseInt(e.target.value) || 50;
        encryptionTimelineSlider.max = keySteps;
        if (timelineSliderVal > keySteps) {
            timelineSliderVal = keySteps;
            encryptionTimelineSlider.value = keySteps;
        }
        runSimulation();
    });

    inetBtn.addEventListener("click", () => {
        // Generate random seed
        const newSeed = Math.floor(Math.random() * 9000000000000000) + 1000000000000000;
        seedInput.value = newSeed;
        seedValue = newSeed.toString();
        runSimulation();
    });

    strengthSlider.addEventListener("input", (e) => {
        const val = parseInt(e.target.value);
        const segments = strengthBars.children;
        for (let i = 0; i < segments.length; i++) {
            if (i < val) {
                segments[i].classList.add("active");
            } else {
                segments[i].classList.remove("active");
            }
        }
        runSimulation();
    });

    seedInput.addEventListener("input", (e) => {
        seedValue = e.target.value;
        runSimulation();
    });

    iterationsSpinner.addEventListener("input", (e) => {
        iterations = parseInt(e.target.value) || 5;
        runSimulation();
    });

    updateRuleToggle.addEventListener("change", (e) => {
        updateRuleActive = e.target.checked;
        runSimulation();
    });

    encryptionTimelineSlider.addEventListener("input", (e) => {
        timelineSliderVal = parseInt(e.target.value);
        // Update timeline progress bar height
        const percent = (timelineSliderVal / keySteps) * 100;
        timelineProgress.style.height = `${percent}%`;
        runSimulation();
    });

    runCalcBtn.addEventListener("click", () => {
        runSimulation();
    });

    // Initial Run
    runSimulation();

    // ─────────────────────────────────────────────────────────────────
    // Simulation Logic
    // ─────────────────────────────────────────────────────────────────

    function runSimulation() {
        // Inputs
        const valA = parseInt(inputA.value) || 0;
        const valB = parseInt(inputB.value) || 0;
        const op = calcOp.value;

        // Constants
        const size = 64;
        const k = 8; // 8 bits

        // 1. Repetition Coding (8-bit to 64-bit)
        const gridA = encodeRepetition(valA, k, size);
        const gridB = encodeRepetition(valB, k, size);

        // 2. IV generation from seed
        const iv = seedToIV(seedValue);

        // Mask starting states
        const startA = gridA ^ iv;
        const startB = gridB ^ iv;

        // 3. Evolve CA Forward
        let lut = getRulesetLUT(ruleset, currentDimension);
        
        let historyA, historyB;
        if (currentDimension === "1D") {
            historyA = evolveReversible1D(startA, iv, lut, radius, size, keySteps);
            historyB = evolveReversible1D(startB, iv, lut, radius, size, keySteps);
        } else {
            historyA = evolveReversible2D(startA, iv, lut, keySteps);
            historyB = evolveReversible2D(startB, iv, lut, keySteps);
        }

        const cipherA = historyA[historyA.length - 1];
        const cipherB = historyB[historyB.length - 1];

        // 4. Homomorphic Evaluation on Ciphertexts
        let evaluatedCipher;
        let expectedMathResult;

        if (op === "xor") {
            evaluatedCipher = cipherA ^ cipherB;
            expectedMathResult = valA ^ valB;
        } else if (op === "or") {
            evaluatedCipher = cipherA | cipherB;
            expectedMathResult = valA | valB;
        } else if (op === "and") {
            evaluatedCipher = cipherA & cipherB;
            expectedMathResult = valA & valB;
        } else if (op === "add") {
            // Simulated 64-bit modular addition
            evaluatedCipher = (cipherA + cipherB) & 0xffffffffffffffffn;
            expectedMathResult = (valA + valB) & 0xff; // 8-bit wrap
        }

        // 5. Decryption & Reverse Evolution
        let recoveredGrid;
        let decryptedVal;

        if (op === "xor") {
            // Reversible CA-HE allows perfect XOR decryption by back-evolution under combined IV = 0
            let decHistory;
            // Back evolve evaluatedCipher
            const c0 = historyA[historyA.length - 2] ^ historyB[historyB.length - 2];
            const c1 = evaluatedCipher;
            
            let recoveredPair;
            if (currentDimension === "1D") {
                recoveredPair = reverseReversible1D(c0, c1, lut, radius, size, keySteps);
            } else {
                recoveredPair = reverseReversible2D(c0, c1, lut, keySteps);
            }
            // Recovered state corresponds to S_0 = (Encoded A ^ IV) ^ (Encoded B ^ IV) = Encoded A ^ Encoded B
            recoveredGrid = recoveredPair[0]; 
            decryptedVal = decodeRepetition(recoveredGrid, k, size);
        } else {
            // Non-linear or add operations violate homomorphism on CA, leading to diffusion noise
            // We simulate the diffusion noise and the resulting error-correction failure
            let decHistory;
            const c0 = (historyA[historyA.length - 2] + historyB[historyB.length - 2]) & 0xffffffffffffffffn;
            const c1 = evaluatedCipher;
            
            let recoveredPair;
            if (currentDimension === "1D") {
                recoveredPair = reverseReversible1D(c0, c1, lut, radius, size, keySteps);
            } else {
                recoveredPair = reverseReversible2D(c0, c1, lut, keySteps);
            }
            
            // Introduce simulated chaotic bit corruption representing HE noise in non-linear ops
            let noiseMask = 0n;
            let prng = valA * 13 + valB * 37;
            for (let i = 0; i < 64; i++) {
                prng = (prng * 1103515245 + 12345) & 0x7fffffff;
                if ((prng % 100) < 35) { // 35% bit-flip probability
                    noiseMask |= (1n << BigInt(i));
                }
            }
            
            recoveredGrid = recoveredPair[0] ^ noiseMask;
            decryptedVal = decodeRepetition(recoveredGrid, k, size);
        }

        // ─────────────────────────────────────────────────────────────────
        // Update Dashboard Visuals
        // ─────────────────────────────────────────────────────────────────

        // Middle column vertical state grids (representing primary simulation A)
        // Adjust indices according to the current keySteps setting
        const step10Idx = Math.floor(keySteps * 0.2);
        const step20Idx = Math.floor(keySteps * 0.4);
        const step40Idx = Math.floor(keySteps * 0.8);

        // Select which state grids to render based on the slider value
        // E.g., if slider is set to 30, cap the history display steps
        const t000State = historyA[0];
        const t010State = historyA[Math.min(step10Idx, timelineSliderVal)];
        const t020State = historyA[Math.min(step20Idx, timelineSliderVal)];
        const t040State = historyA[Math.min(step40Idx, timelineSliderVal)];
        const t050State = historyA[Math.min(keySteps, timelineSliderVal)];

        // Render timeline step grids
        renderTimelineGrid("grid-t000", t000State);
        renderTimelineGrid("grid-t010", t010State);
        renderTimelineGrid("grid-t020", t020State);
        renderTimelineGrid("grid-t040", t040State);
        renderTimelineGrid("grid-t050", t050State);

        // Update timeline step labels
        document.querySelector(".chronology-step:nth-child(1) .step-label").innerText = `T=000`;
        document.querySelector(".chronology-step:nth-child(3) .step-label").innerText = `T=${pad3(Math.min(step10Idx, timelineSliderVal))}`;
        document.querySelector(".chronology-step:nth-child(5) .step-label").innerText = `T=${pad3(Math.min(step20Idx, timelineSliderVal))}`;
        document.querySelector(".chronology-step:nth-child(7) .step-label").innerText = `T=${pad3(Math.min(step40Idx, timelineSliderVal))}`;
        document.querySelector(".chronology-step:nth-child(9) .step-label").innerText = `T=${pad3(Math.min(keySteps, timelineSliderVal))}`;

        // ─────────────────────────────────────────────────────────────────
        // Update Sandbox & Explainer Details
        // ─────────────────────────────────────────────────────────────────
        calcResultVal.innerText = decryptedVal;

        // Render binary text labels
        document.getElementById("text-bin-a").innerText = valA.toString(2).padStart(8, '0');
        document.getElementById("text-bin-b").innerText = valB.toString(2).padStart(8, '0');
        document.getElementById("text-bin-iv").innerText = iv.toString(2).substring(0, 8) + "...";

        // Render step flow sequences
        renderBitSequence("flow-seq-enc-a", gridA);
        renderBitSequence("flow-seq-enc-b", gridB);
        renderBitSequence("flow-seq-iv", iv);
        renderBitSequence("flow-seq-plain-a", gridA);
        renderBitSequence("flow-seq-init-a", startA);
        renderBitSequence("flow-seq-diff-t000", startA);
        renderBitSequence("flow-seq-diff-t050", cipherA);
        renderBitSequence("flow-seq-eval-ca", cipherA);
        renderBitSequence("flow-seq-eval-cb", cipherB);
        renderBitSequence("flow-seq-eval-res", evaluatedCipher);
        renderBitSequence("flow-seq-dec-grid", recoveredGrid);
        
        // Show decoded binary representation
        document.getElementById("flow-dec-text").innerText = decryptedVal.toString(2).padStart(8, '0');

        // Check if verified
        if (decryptedVal === expectedMathResult) {
            sandboxBadge.className = "sandbox-status-badge";
            sandboxBadge.style.backgroundColor = "rgba(16, 185, 129, 0.15)";
            sandboxBadge.style.borderColor = "rgba(16, 185, 129, 0.4)";
            sandboxBadge.style.color = "#10b981";
            sandboxBadge.innerText = "VERIFIED";
        } else {
            sandboxBadge.className = "sandbox-status-badge";
            sandboxBadge.style.backgroundColor = "rgba(239, 68, 68, 0.15)";
            sandboxBadge.style.borderColor = "rgba(239, 68, 68, 0.4)";
            sandboxBadge.style.color = "#ef4444";
            sandboxBadge.innerText = "NOISY / FAILED";
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // Helper Functions
    // ─────────────────────────────────────────────────────────────────

    function pad3(num) {
        return num.toString().padStart(3, '0');
    }

    function encodeRepetition(val, k, n) {
        const r = Math.floor(n / k); // 8 bits repeated 8 times
        let grid = 0n;
        for (let bitIdx = 0; bitIdx < k; bitIdx++) {
            let bitVal = BigInt((val >> bitIdx) & 1);
            for (let j = 0; j < r; j++) {
                let idx = BigInt(bitIdx * r + j);
                if (idx < BigInt(n)) {
                    grid |= (bitVal << idx);
                }
            }
        }
        return grid;
    }

    function decodeRepetition(val, k, n) {
        const r = Math.floor(n / k);
        let decoded = 0;
        for (let bitIdx = 0; bitIdx < k; bitIdx++) {
            let start = bitIdx * r;
            let end = (bitIdx === k - 1) ? n : (bitIdx + 1) * r;
            let ones = 0;
            let count = end - start;
            for (let i = start; i < end; i++) {
                if (((val >> BigInt(i)) & 1n) !== 0n) {
                    ones++;
                }
            }
            if (ones > count / 2) {
                decoded |= (1 << bitIdx);
            }
        }
        return decoded;
    }

    function seedToIV(seedStr) {
        let hash = 0n;
        for (let i = 0; i < seedStr.length; i++) {
            hash = (hash * 33n + BigInt(seedStr.charCodeAt(i))) & 0xffffffffffffffffn;
        }
        let state = hash;
        let iv = 0n;
        for (let i = 0; i < 64; i++) {
            state = (state * 6364136223846793005n + 1442695040888963407n) & 0xffffffffffffffffn;
            if (state % 2n === 1n) {
                iv |= (1n << BigInt(i));
            }
        }
        return iv;
    }

    function getRulesetLUT(ruleset, dimension) {
        if (dimension === "1D") {
            if (ruleset === "30") return 30n;
            if (ruleset === "90") return 90n;
            // Evolved Rule 43/36 LUT representation
            return 165n; 
        } else {
            // 2D rules require a 32-bit LUT (e.g. von Neumann neighborhood)
            if (ruleset === "30") return 0x1e2d3c4bn; // mock 2D chaotic rule
            if (ruleset === "90") return 0x5a5a5a5an; // mock 2D linear rule
            return 0x8f3c71adn; // evolved 2D CA-HE rule
        }
    }

    // 1D CA Rule Application
    function applyRule1D(state, ruleLut, radius, size) {
        let newState = 0n;
        const sizeBig = BigInt(size);
        const ruleBig = BigInt(ruleLut);
        for (let i = 0n; i < sizeBig; i++) {
            let val = 0n;
            for (let r = -radius; r <= radius; r++) {
                let shift = (i + BigInt(r) + sizeBig) % sizeBig;
                let bit = (state >> shift) & 1n;
                val = (val << 1n) | bit;
            }
            let output = (ruleBig >> val) & 1n;
            newState |= (output << i);
        }
        return newState;
    }

    // Reversible 1D second-order evolve
    function evolveReversible1D(start, iv, ruleLut, radius, size, steps) {
        let p = start ^ iv; // S_{-1} = start ^ iv
        let c = start;      // S_0 = start
        let history = [c];
        for (let t = 0; t < steps; t++) {
            let next = applyRule1D(c, ruleLut, radius, size) ^ p;
            p = c;
            c = next;
            history.push(c);
        }
        return history;
    }

    // Reversible 1D second-order reverse
    function reverseReversible1D(c0, c1, ruleLut, radius, size, steps) {
        let next = c1;
        let curr = c0;
        for (let t = 0; t < steps; t++) {
            let prev = applyRule1D(curr, ruleLut, radius, size) ^ next;
            next = curr;
            curr = prev;
        }
        return [curr, next];
    }

    // 2D CA Rule Application (8x8 Von Neumann)
    function applyRule2D(state, ruleLut) {
        let newState = 0n;
        const ruleBig = BigInt(ruleLut);
        for (let y = 0n; y < 8n; y++) {
            for (let x = 0n; x < 8n; x++) {
                let u = (state >> (((y - 1n + 8n) % 8n) * 8n + x)) & 1n;
                let d = (state >> (((y + 1n) % 8n) * 8n + x)) & 1n;
                let l = (state >> (y * 8n + (x - 1n + 8n) % 8n)) & 1n;
                let r = (state >> (y * 8n + (x + 1n) % 8n)) & 1n;
                let c = (state >> (y * 8n + x)) & 1n;

                let index = (u << 4n) | (d << 3n) | (l << 2n) | (r << 1n) | c;
                let output = (ruleBig >> index) & 1n;
                newState |= (output << (y * 8n + x));
            }
        }
        return newState;
    }

    // Reversible 2D second-order evolve
    function evolveReversible2D(start, iv, ruleLut, steps) {
        let p = start ^ iv;
        let c = start;
        let history = [c];
        for (let t = 0; t < steps; t++) {
            let next = applyRule2D(c, ruleLut) ^ p;
            p = c;
            c = next;
            history.push(c);
        }
        return history;
    }

    // Reversible 2D second-order reverse
    function reverseReversible2D(c0, c1, ruleLut, steps) {
        let next = c1;
        let curr = c0;
        for (let t = 0; t < steps; t++) {
            let prev = applyRule2D(curr, ruleLut) ^ next;
            next = curr;
            curr = prev;
        }
        return [curr, next];
    }

    // Render 8x8 Text Grid
    function renderTimelineGrid(containerId, state) {
        const container = document.getElementById(containerId);
        if (!container) return;
        container.innerHTML = "";
        for (let y = 0; y < 8; y++) {
            for (let x = 0; x < 8; x++) {
                let idx = y * 8 + x;
                let bit = (state >> BigInt(idx)) & 1n;
                const span = document.createElement("span");
                span.className = bit ? 'one' : 'zero';
                span.innerText = bit.toString();
                container.appendChild(span);
            }
        }
    }

    // Render 64-bit Binary Sequence Spans
    function renderBitSequence(containerId, val) {
        const container = document.getElementById(containerId);
        if (!container) return;
        container.innerHTML = "";
        for (let i = 63; i >= 0; i--) {
            const bit = (val >> BigInt(i)) & 1n;
            const span = document.createElement("span");
            span.className = `bit-cell ${bit ? 'one' : 'zero'}`;
            span.innerText = bit.toString();
            container.appendChild(span);
        }
    }
});
