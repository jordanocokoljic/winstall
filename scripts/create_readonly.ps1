# Create the directories, readonly_dir & readonly_dir\b
$cwd = Get-Location
$parentDir = Join-Path -Path $cwd -ChildPath "readonly_dir"
$readonlyDir = Join-Path -Path $parentDir -ChildPath "b"

Remove-Item -Path $parentDir -Recurse

if (!(Test-Path $parentDir)) {
    New-Item -ItemType Directory -Path $parentDir
}

if (!(Test-Path $readonlyDir)) {
    New-Item -ItemType Directory -Path $readonlyDir
}

# Get the current user
$user = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name

# Create a deny rule and set it on the directory readonly_dir\b
$denyRule = New-Object System.Security.AccessControl.FileSystemAccessRule(
    $user,
    "Write",
    "ContainerInherit,ObjectInherit",
    "None",
    "Deny"
)

$acl = Get-Acl -Path $readonlyDir
$acl.SetAccessRule($denyRule)
Set-Acl -Path $readonlyDir -AclObject $acl