# check_task.ps1 - Verifies the dwm_eotf_rs scheduled task is configured correctly.
# Run this AFTER registering the task with: dwm_eotf_rs.exe --startup

$taskName = "dwm_eotf_rs"
$taskPath = "\Users\$env:USERNAME\"

Write-Host ""
Write-Host "======================================" -ForegroundColor Cyan
Write-Host "  dwm_eotf_rs Task Scheduler Checker" -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""

# --- Find the task ---
$task = Get-ScheduledTask -TaskName $taskName -TaskPath $taskPath -ErrorAction SilentlyContinue

if (-not $task) {
    # Check if it exists at the old root path
    $legacyTask = Get-ScheduledTask -TaskName $taskName -TaskPath "\" -ErrorAction SilentlyContinue
    if ($legacyTask) {
        Write-Host "[!] Task found at ROOT path '\$taskName' (legacy location)" -ForegroundColor Yellow
        Write-Host "    Expected per-user path: $taskPath" -ForegroundColor Yellow
        $task = $legacyTask
    } else {
        Write-Host "[X] Task '$taskName' not found!" -ForegroundColor Red
        Write-Host "    Register it first: dwm_eotf_rs.exe --startup" -ForegroundColor Gray
        exit 1
    }
}

Write-Host "Task found: $($task.TaskPath)$($task.TaskName)" -ForegroundColor Gray
Write-Host ""

$allPassed = $true

# --- Check 1: Battery settings ---
Write-Host "--- Check 1: Battery Settings ---" -ForegroundColor White
$settings = $task.Settings

$allowBattery = -not $settings.DisallowStartIfOnBatteries
$dontStop     = -not $settings.StopIfGoingOnBatteries

if ($allowBattery) {
    Write-Host "  [PASS] AllowStartIfOnBatteries = True" -ForegroundColor Green
} else {
    Write-Host "  [FAIL] DisallowStartIfOnBatteries is ON - task won't start on battery!" -ForegroundColor Red
    $allPassed = $false
}

if ($dontStop) {
    Write-Host "  [PASS] StopIfGoingOnBatteries = False" -ForegroundColor Green
} else {
    Write-Host "  [FAIL] StopIfGoingOnBatteries is ON - task will stop when switching to battery!" -ForegroundColor Red
    $allPassed = $false
}

# --- Check 2: Execution time limit ---
Write-Host ""
Write-Host "--- Check 2: Execution Time Limit ---" -ForegroundColor White
$timeLimit = $settings.ExecutionTimeLimit

if ($timeLimit -eq "PT0S" -or $timeLimit -eq $null -or $timeLimit -eq "") {
    Write-Host "  [PASS] ExecutionTimeLimit = '$timeLimit' (unlimited)" -ForegroundColor Green
} else {
    Write-Host "  [FAIL] ExecutionTimeLimit = '$timeLimit' - task will be killed after this duration!" -ForegroundColor Red
    Write-Host "         Expected: 'PT0S' (unlimited)" -ForegroundColor Gray
    $allPassed = $false
}

# --- Check 3: Per-user task ---
Write-Host ""
Write-Host "--- Check 3: Per-User Configuration ---" -ForegroundColor White

$expectedPath = "\Users\$env:USERNAME\"
if ($task.TaskPath -eq $expectedPath) {
    Write-Host "  [PASS] TaskPath = '$($task.TaskPath)' (per-user)" -ForegroundColor Green
} else {
    Write-Host "  [FAIL] TaskPath = '$($task.TaskPath)' - expected '$expectedPath'" -ForegroundColor Red
    $allPassed = $false
}

$principal = $task.Principal
Write-Host "  Principal UserId:    $($principal.UserId)" -ForegroundColor Gray
Write-Host "  Principal LogonType: $($principal.LogonType)" -ForegroundColor Gray
Write-Host "  Principal RunLevel:  $($principal.RunLevel)" -ForegroundColor Gray

if ($principal.LogonType -eq "Interactive") {
    Write-Host "  [PASS] LogonType = Interactive" -ForegroundColor Green
} else {
    $lt = $principal.LogonType
    Write-Host "  [FAIL] LogonType = '$lt' - expected 'Interactive'" -ForegroundColor Red
    $allPassed = $false
}

if ($principal.RunLevel -eq "Highest") {
    Write-Host "  [PASS] RunLevel = Highest (admin)" -ForegroundColor Green
} else {
    $rl = $principal.RunLevel
    Write-Host "  [FAIL] RunLevel = '$rl' - expected 'Highest'" -ForegroundColor Red
    $allPassed = $false
}

# Check trigger is per-user
$triggers = $task.Triggers
$logonTrigger = $triggers | Where-Object { $_.CimClass.CimClassName -eq "MSFT_TaskLogonTrigger" }
if ($logonTrigger) {
    $triggerUser = $logonTrigger.UserId
    if ($triggerUser -and $triggerUser -like "*$env:USERNAME*") {
        Write-Host "  [PASS] Logon trigger is for user: $triggerUser" -ForegroundColor Green
    } elseif (-not $triggerUser -or $triggerUser -eq "") {
        Write-Host "  [FAIL] Logon trigger has no user filter - fires for ANY user!" -ForegroundColor Red
        $allPassed = $false
    } else {
        Write-Host "  [WARN] Logon trigger user: $triggerUser (current user: $env:USERNAME)" -ForegroundColor Yellow
    }
} else {
    Write-Host "  [FAIL] No logon trigger found!" -ForegroundColor Red
    $allPassed = $false
}

# --- Summary ---
Write-Host ""
Write-Host "======================================" -ForegroundColor Cyan
if ($allPassed) {
    Write-Host "  ALL CHECKS PASSED" -ForegroundColor Green
} else {
    Write-Host "  SOME CHECKS FAILED" -ForegroundColor Red
}
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""
