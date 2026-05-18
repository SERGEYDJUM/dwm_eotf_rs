$task_path = "\Users\" + $env:USERNAME + "\"
Get-ScheduledTask -TaskName "dwm_eotf_rs" -TaskPath $task_path -ErrorAction SilentlyContinue
