param(
    [string]$Device,
    [string[]]$Features
)

function Show-Usage {
    Write-Host "Usage:"
    Write-Host "  .\build.ps1                                 # build all devices"
    Write-Host "  .\build.ps1 -Device esp32                   # build single device"
    Write-Host "  .\build.ps1 -Features log,max-cpu-frequency # build with specified features"
    Write-Host ""
    Write-Host "Notes:"
    Write-Host "  - The -Features parameter accepts multiple values or a comma-separated string."
    exit 1
}

# Remove existing YAML outputs (ignore errors if none exist)
Remove-Item -Path output/*.yaml -Force -ErrorAction SilentlyContinue

# map device -> target
$deviceMap = @{
    "esp32"  = "xtensa-esp32-none-elf"
    "esp32s2" = "xtensa-esp32s2-none-elf"
    "esp32s3" = "xtensa-esp32s3-none-elf"
    "esp32c2" = "riscv32imc-unknown-none-elf"
    "esp32c3" = "riscv32imc-unknown-none-elf"
    "esp32c5" = "riscv32imac-unknown-none-elf"
    "esp32c6" = "riscv32imac-unknown-none-elf"
    # "esp32c61" = "riscv32imac-unknown-none-elf"
    "esp32h2" = "riscv32imac-unknown-none-elf"
}

# Validate device (if specified) and build list
if ($Device) {
    if (-not $deviceMap.ContainsKey($Device)) {
        Write-Error "Unknown device: $Device. Valid devices are: $($deviceMap.Keys -join ', ')"
        Show-Usage
    }
    $devicesToBuild = @($Device)
} else {
    $devicesToBuild = $deviceMap.Keys
}

# Normalize features passed via -Features (accepts array items and comma-separated strings)
$parsedFeatures = @()
if ($Features) {
    $joined = $Features -join ','
    $parsedFeatures = $joined -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ -ne '' }
}

# Deduplicate while preserving order
$finalFeatures = @()
foreach ($f in $parsedFeatures) {
    if (-not ($finalFeatures -contains $f)) {
        $finalFeatures += $f
    }
}

# Build loop
foreach ($d in $devicesToBuild) {
    $targetPath = ("target/{0}/release/esp-flashloader" -f ( $deviceMap[$d] ))
    Write-Host "=== Building $d ==="

    # Build cargo arguments
    $cargoArgs = @("+esp", $d)

    if ($finalFeatures.Count -gt 0) {
        $cargoArgs += "--features"
        $cargoArgs += ($finalFeatures -join ",")
        Write-Host ("running command: cargo {0}" -f ($cargoArgs -join ' '))
    } else {
        Write-Host ("running command: cargo {0}" -f ($cargoArgs -join ' '))
    }

    # Run cargo
    $cargoExit = & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Error "cargo failed for device $d (exit code $LASTEXITCODE). Aborting."
        exit $LASTEXITCODE
    }

    # Run target-gen to produce YAML
    $outYaml = "output/$d.yaml"
    Write-Host "Generating YAML: $outYaml"
    & target-gen elf $targetPath $outYaml --name "$d-flashloader"
    if ($LASTEXITCODE -ne 0) {
        Write-Error "target-gen failed for device $d (exit code $LASTEXITCODE). Aborting."
        exit $LASTEXITCODE
    }

    Write-Host ""
}

Write-Host "All done."
