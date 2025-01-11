SET currentPath=%~dp0

sc.exe create gameWorldBackupService binPath= "%currentPath%\seven_days_backer.exe" start= auto displayName= "Game World Backup Service"

sc.exe start backupService
