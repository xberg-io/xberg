$ErrorActionPreference = "Stop"

$taskVersion = $args[0]
if ([string]::IsNullOrWhiteSpace($taskVersion)) {
  throw "Usage: windows.ps1 <taskVersion>"
}

$taskBinDir = "$env:USERPROFILE\AppData\Local\task"
New-Item -ItemType Directory -Force -Path $taskBinDir | Out-Null

$taskExe = "$taskBinDir\task.exe"

if (-not (Test-Path $taskExe)) {
  $zipPath = "$taskBinDir\task.zip"
  $directUrl = "https://github.com/go-task/task/releases/download/v$taskVersion/task_windows_amd64.zip"

  try {
    Invoke-WebRequest -Uri $directUrl -OutFile $zipPath
  } catch {
    $releases = "https://api.github.com/repos/go-task/task/releases/tags/v$taskVersion"
    $headers = @{}
    if ($env:GITHUB_TOKEN) {
      $headers["Authorization"] = "Bearer $env:GITHUB_TOKEN"
      $headers["X-GitHub-Api-Version"] = "2022-11-28"
    }
    $release = Invoke-RestMethod -Uri $releases -Headers $headers
    $asset = $release.assets | Where-Object { $_.name -match "windows_amd64\.zip" } | Select-Object -First 1

    if (-not $asset) {
      throw "Could not find Windows amd64 release for Task v$taskVersion"
    }

    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $zipPath
  }

  Expand-Archive -Path $zipPath -DestinationPath $taskBinDir -Force
  Remove-Item $zipPath
}

"$taskBinDir" | Out-File -FilePath $env:GITHUB_PATH -Encoding utf8 -Append
