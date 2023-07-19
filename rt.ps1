#!/usr/bin/env pwsh

# Thanks to Moon for the original script:
# https://moonrepo.dev/

$DownloadUrl = "https://github.com/MoskalykA/rt/releases/latest/download/rt.exe"

$InstallDir = "${Home}\.rt\bin"
$BinPath = "${InstallDir}\rt.exe"

if (!(Test-Path $InstallDir)) {
  New-Item $InstallDir -ItemType Directory | Out-Null
}

curl.exe -Lo $BinPath $DownloadUrl

# Windows doesn't support a "shared binaries" type of folder,
# so instead of symlinking, we add the install dir to $PATH.
$User = [System.EnvironmentVariableTarget]::User
$Path = [System.Environment]::GetEnvironmentVariable('Path', $User)

if (!(";${Path};".ToLower() -like "*;${InstallDir};*".ToLower())) {
  [System.Environment]::SetEnvironmentVariable('Path', "${InstallDir};${Path}", $User)
  $Env:Path = "${InstallDir};${Env:Path}"
}

Write-Output "Successfully installed rt to ${BinPath}"
Write-Output "Run 'rt --help' to get started!"
