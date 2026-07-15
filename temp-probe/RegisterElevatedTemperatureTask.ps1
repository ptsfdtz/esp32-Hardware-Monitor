param(
  [Parameter(Mandatory = $false)]
  [string]$ProgramDirectory,

  [Parameter(Mandatory = $false)]
  [string]$AgentPath,

  [switch]$ValidateOnly,

  [switch]$Remove
)

$ErrorActionPreference = 'Stop'

if ($ValidateOnly) {
  Write-Output 'task script parsed'
  exit 0
}

$taskFolderPath = '\ESP32HardwareMonitor'
$taskName = 'ElevatedTemperatureAgent'
# Only SYSTEM and the Administrators group may change this task or its folder.
$sddl = 'O:SYG:SYD:P(A;;FA;;;SY)(A;;FA;;;BA)'
$taskCreateOrUpdate = 6
$taskDontAddPrincipalAce = 0x10
$taskLogonInteractiveToken = 3
$taskRunLevelHighest = 1
$taskTriggerLogon = 9
$taskActionExec = 0
$taskInstancesIgnoreNew = 2
$securityInformationOwnerGroupDacl = 7

if ($Remove) {
  $service = New-Object -ComObject 'Schedule.Service'
  $service.Connect()
  $root = $service.GetFolder('\')
  try {
    $folder = $service.GetFolder($taskFolderPath)
    try {
      $task = $folder.GetTask($taskName)
      $task.Stop(0)
      $folder.DeleteTask($taskName, 0)
    }
    catch {
      if ($_.Exception.Message -notmatch 'cannot find|not exist') {
        throw
      }
    }
    try {
      $root.DeleteFolder('ESP32HardwareMonitor', 0)
    }
    catch {
    }
  }
  catch {
    if ($_.Exception.Message -notmatch 'cannot find|not exist') {
      throw
    }
  }
  Write-Output 'elevated temperature task removed'
  exit 0
}

if (!(Test-Path -LiteralPath $AgentPath -PathType Leaf)) {
  throw "Elevated temperature agent not found: $AgentPath"
}
if ([string]::IsNullOrWhiteSpace($ProgramDirectory)) {
  throw 'ProgramDirectory is required when registering the elevated temperature task.'
}

$sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$service = New-Object -ComObject 'Schedule.Service'
$service.Connect()
$root = $service.GetFolder('\')

try {
  $folder = $service.GetFolder($taskFolderPath)
}
catch {
  $folder = $root.CreateFolder('ESP32HardwareMonitor', $sddl)
}

$folder.SetSecurityDescriptor($sddl, 0)

$definition = $service.NewTask(0)
$definition.RegistrationInfo.Author = 'ESP32 Hardware Monitor'
$definition.RegistrationInfo.Description = 'Protected elevated CPU/GPU temperature sampler.'
$definition.Settings.Enabled = $true
$definition.Settings.Hidden = $true
$definition.Settings.AllowDemandStart = $true
$definition.Settings.StartWhenAvailable = $true
$definition.Settings.DisallowStartIfOnBatteries = $false
$definition.Settings.StopIfGoingOnBatteries = $false
$definition.Settings.ExecutionTimeLimit = 'PT0S'
$definition.Settings.RestartCount = 3
$definition.Settings.RestartInterval = 'PT1M'
$definition.Settings.MultipleInstances = $taskInstancesIgnoreNew

$trigger = $definition.Triggers.Create($taskTriggerLogon)
$trigger.Id = 'Logon'
$trigger.UserId = $sid
$trigger.Delay = 'PT10S'
$trigger.Enabled = $true

$definition.Principal.Id = 'CurrentUser'
$definition.Principal.UserId = $sid
$definition.Principal.LogonType = $taskLogonInteractiveToken
$definition.Principal.RunLevel = $taskRunLevelHighest

$action = $definition.Actions.Create($taskActionExec)
$action.Path = $AgentPath
$action.Arguments = '--elevated-agent'
$action.WorkingDirectory = $ProgramDirectory

$flags = $taskCreateOrUpdate -bor $taskDontAddPrincipalAce
$registered = $folder.RegisterTaskDefinition(
  $taskName,
  $definition,
  $flags,
  $sid,
  $null,
  $taskLogonInteractiveToken,
  $sddl
)
$registered.SetSecurityDescriptor($sddl, $taskDontAddPrincipalAce)

$actualFolderSddl = $folder.GetSecurityDescriptor($securityInformationOwnerGroupDacl)
$actualTaskSddl = $registered.GetSecurityDescriptor($securityInformationOwnerGroupDacl)
if ($actualFolderSddl -notmatch 'O:SY' -or $actualFolderSddl -notmatch ';;;SY' -or $actualFolderSddl -notmatch ';;;BA') {
  throw "Task folder security descriptor was not applied: $actualFolderSddl"
}
if ($actualTaskSddl -notmatch 'O:SY' -or $actualTaskSddl -notmatch ';;;SY' -or $actualTaskSddl -notmatch ';;;BA' -or $actualTaskSddl -match [regex]::Escape($sid)) {
  throw "Task security descriptor was not applied: $actualTaskSddl"
}

$registered.Run($null) | Out-Null
Write-Output "registered=$taskFolderPath\\$taskName"
