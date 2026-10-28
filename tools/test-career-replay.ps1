param(
    [string]$GameRoot=$env:SKATE2_GAME_ROOT,
    [string]$SourceSave,
    [int]$TimeoutSeconds=600,
    [switch]$Media
)
$ErrorActionPreference='Stop'
$repo=Split-Path -Parent $PSScriptRoot
$exe=Join-Path $repo 'target\release\skate2-pc.exe'
if(-not $SourceSave){$SourceSave=Join-Path $repo 'saves\SKATER.P'}
if(-not(Test-Path -LiteralPath $SourceSave -PathType Leaf)){throw "Existing Career profile required: $SourceSave"}
if(Get-Process -Name skate2-pc -ErrorAction SilentlyContinue){throw 'Close the running PC game before this replay test.'}
$run=Join-Path $repo ('.scratch\replay-entry-test-'+(Get-Date -Format 'yyyyMMdd-HHmmss-fff'))
if(Test-Path -LiteralPath $run){throw "Run directory already exists: $run"}
$save=Join-Path $run 'save'
New-Item -ItemType Directory -Path $save | Out-Null
Copy-Item -LiteralPath $SourceSave -Destination (Join-Path $save 'SKATER.P')
$before=(Get-FileHash -LiteralPath $SourceSave -Algorithm SHA256).Hash
$stdout=Join-Path $run 'stdout.log';$stderr=Join-Path $run 'stderr.log';$capture=Join-Path $run 'replay-roundtrip.png'
$keys=@('SKATE2_GAME_ROOT','SKATE2_SAVE_ROOT','SKATE2_DIAGNOSTIC_REPLAY_ENTRY','SKATE2_DIAGNOSTIC_REPLAY_MEDIA','SKATE2_DIAGNOSTIC_BACKGROUND_WINDOW','SKATE2_DIAGNOSTIC_WINDOW_SIZE',
    'SKATE2_DIAGNOSTIC_CAPTURE_EVERY','SKATE2_DIAGNOSTIC_CAPTURE_FROM','SKATE2_DIAGNOSTIC_CAPTURE_COUNT')
$old=@{};foreach($key in $keys){$old[$key]=[Environment]::GetEnvironmentVariable($key,'Process')}
try {
    $env:SKATE2_GAME_ROOT=$GameRoot;$env:SKATE2_SAVE_ROOT=$save
    $env:SKATE2_DIAGNOSTIC_REPLAY_ENTRY='1';$env:SKATE2_DIAGNOSTIC_BACKGROUND_WINDOW='1';$env:SKATE2_DIAGNOSTIC_WINDOW_SIZE='1280x720'
    if($Media){$env:SKATE2_DIAGNOSTIC_REPLAY_MEDIA='1'}else{Remove-Item Env:SKATE2_DIAGNOSTIC_REPLAY_MEDIA -ErrorAction SilentlyContinue}
    $env:SKATE2_DIAGNOSTIC_CAPTURE_EVERY='2';$env:SKATE2_DIAGNOSTIC_CAPTURE_FROM='10';$env:SKATE2_DIAGNOSTIC_CAPTURE_COUNT='60'
    $arguments=@('--live-skating','--live-controls-demo','career','--live-frames','1000000','--live-capture',('"'+$capture+'"'))
    $process=Start-Process -FilePath $exe -WorkingDirectory $repo -ArgumentList $arguments -WindowStyle Hidden -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru
    $null=$process.Handle # Retain the native handle so Windows PowerShell preserves ExitCode.
    "Replay test PID $($process.Id); artifacts: $run"
    $deadline=(Get-Date).AddSeconds($TimeoutSeconds)
    while(-not $process.WaitForExit(1000)) {
        if((Get-Date) -gt $deadline){Stop-Process -Id $process.Id;throw "Replay test timed out; see $stderr"}
    }
    $process.Refresh()
    $log=Get-Content -LiteralPath $stderr -Raw
    if($process.ExitCode -ne 0 -or $log -notmatch 'REPLAY_TEST PASS:' -or -not(Test-Path -LiteralPath $capture)) {
        throw "Replay test failed (exit $($process.ExitCode)); see $stderr"
    }
    if((Get-FileHash -LiteralPath $SourceSave -Algorithm SHA256).Hash -ne $before){throw 'Source profile changed during diagnostic'}
    [ordered]@{pass=$true;media=[bool]$Media;exit=$process.ExitCode;executable=$exe;sha256=(Get-FileHash $exe).Hash;sourceProfileUnchanged=$true;capture=$capture} |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $run 'result.json')
    "PASS: $capture"
} finally {foreach($key in $keys){[Environment]::SetEnvironmentVariable($key,$old[$key],'Process')}}
