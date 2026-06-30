$task_path = "\Users\" + $env:USERNAME + "\"
Unregister-ScheduledTask -TaskName "dwm_eotf_rs" -TaskPath $task_path -Confirm:$false
