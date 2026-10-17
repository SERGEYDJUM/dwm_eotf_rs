$task_path = "\" + $env:USERDOMAIN + "\" + $env:USERNAME + "\"
Get-ScheduledTask -TaskName "dwm_eotf_rs" -TaskPath $task_path
