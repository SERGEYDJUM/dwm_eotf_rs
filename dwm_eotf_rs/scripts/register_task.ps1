$exe_path = "{INPUT_app_path}"
$arguments = "{INPUT_app_args}"

$task_path = $env:USERDOMAIN + "\" + $env:USERNAME
$description = "Patches DWM EOTF when user logs in"
$action = New-ScheduledTaskAction -Execute $exe_path -Argument $arguments
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $task_path
$principal = New-ScheduledTaskPrincipal -UserId $task_path -LogonType Interactive -RunLevel Highest

$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1)
$settings.ExecutionTimeLimit = "PT0S"

Register-ScheduledTask -TaskName "dwm_eotf_rs" -TaskPath $task_path -Action $action -Trigger $trigger -Settings $settings -Principal $principal -Description $description -Force
