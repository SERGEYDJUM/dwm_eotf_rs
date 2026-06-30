$exe_path = "{INPUT_app_path}"
$arguments = "{INPUT_app_args}"

$description = "Patches DWM EOTF when user logs in"
$action = New-ScheduledTaskAction -Execute $exe_path -Argument $arguments
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $env:USERNAME
$principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive -RunLevel Highest
$task_path = "\Users\" + $env:USERNAME

$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1)
$settings.ExecutionTimeLimit = "PT0S"

Register-ScheduledTask -TaskName "dwm_eotf_rs" -TaskPath $task_path -Action $action -Trigger $trigger -Settings $settings -Principal $principal -Description $description -Force
