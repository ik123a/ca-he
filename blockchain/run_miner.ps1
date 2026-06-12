$env:Path = [System.Environment]::GetEnvironmentVariable('Path', 'User') + ';' + [System.Environment]::GetEnvironmentVariable('Path', 'Machine')

Write-Host "======================================================================"
Write-Host "CA-HE PROOF-OF-EVOLUTION MINER PIPELINE"
Write-Host "======================================================================"

# 1. Fetch challenge seed
Write-Host "Fetching challenge seed from smart contract..."
$output = npx hardhat run scripts/get_seed.js --network localhost
# Get the last line of output which is the seed hex
$lines = $output -split "`r`n"
$seed = $lines[-1].Trim()

if (-not $seed.StartsWith("0x")) {
    Write-Host "Error: Could not retrieve challenge seed. Output was: $output"
    Exit 1
}

Write-Host "Challenge Seed: $seed"
Write-Host ""

# 2. Run Rust search
Write-Host "Starting parallelized evolutionary search (Rust)..."
$currentDir = Get-Location
Set-Location ../rust
powershell -ExecutionPolicy Bypass -File .\run_search2d.ps1 $seed
Set-Location $currentDir

Write-Host ""
# 3. Submit rule to registry
Write-Host "Submitting discovered rule to registry..."
npx hardhat run scripts/submit_rule.js --network localhost
