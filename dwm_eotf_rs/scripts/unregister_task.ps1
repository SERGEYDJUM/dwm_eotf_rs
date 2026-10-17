$task_path = "\" + $env:USERDOMAIN + "\" + $env:USERNAME + "\"
Unregister-ScheduledTask -TaskName "dwm_eotf_rs" -TaskPath $task_path -Confirm:$false
