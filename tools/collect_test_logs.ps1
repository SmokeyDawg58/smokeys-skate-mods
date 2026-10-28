param(
    [string]$LogRoot = '',
    [string]$RunDirectory = '',
    [string]$Destination = ''
)
$ErrorActionPreference = 'Stop'
if (-not $RunDirectory) {
    if (-not $LogRoot) {
        if ($env:SKATE2_TEST_LOG_ROOT) { $LogRoot = $env:SKATE2_TEST_LOG_ROOT }
        elseif ($env:SKATE2_SAVE_ROOT) { $LogRoot = Join-Path $env:SKATE2_SAVE_ROOT 'test_logs' }
        else { $LogRoot = Join-Path (Get-Location).Path 'saves\test_logs' }
    }
    $run = Get-ChildItem -LiteralPath $LogRoot -Directory |
        Where-Object { $_.Name -like 'run-*' } |
        Sort-Object Name -Descending | Select-Object -First 1
    if (-not $run) { throw "No test runs found in $LogRoot" }
    $RunDirectory = $run.FullName
}
$resolved = (Resolve-Path -LiteralPath $RunDirectory).Path
if (-not (Test-Path -LiteralPath (Join-Path $resolved 'build.txt') -PathType Leaf)) {
    throw 'Select a game test run directory containing build.txt.'
}
if (-not $Destination) { $Destination = "$resolved.zip" }
if (Test-Path -LiteralPath $Destination) { throw "Report already exists: $Destination" }
$files = Get-ChildItem -LiteralPath $resolved -File |
    Where-Object { $_.Name -match '^(build\.txt|executable\.sha256|events-\d+\.jsonl|failure-\d+\.jsonl|logging-error\.txt)$' }
if (-not $files) { throw 'No diagnostic files found.' }
Compress-Archive -LiteralPath $files.FullName -DestinationPath $Destination -CompressionLevel Optimal
Write-Output "Test report: $((Resolve-Path -LiteralPath $Destination).Path)"
