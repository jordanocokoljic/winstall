# Create readonly directory
$cwd = Get-Location
$readonlyDir = Join-Path -Path $cwd -ChildPath 'readonly_directory'
New-Item -ItemType Directory -Path $readonlyDir
New-Item -ItemType File -Path (Join-Path -Path $readonlyDir -ChildPath 'file.txt')

# Get the current user
$user = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name

# Create a deny rule and set it on the directory readonly_dir\b
$deny = New-Object System.Security.AccessControl.FileSystemAccessRule(
    $user,
    "FullControl",
    "ContainerInherit,ObjectInherit",
    "None",
    "Deny"
)

# Update ACL with deny rule
$acl = Get-Acl -Path $readonlyDir
$acl.SetAccessRule($deny)
Set-Acl -Path $readonlyDir -AclObject $acl