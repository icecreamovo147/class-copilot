param(
  [string]$BundleRoot = "src-tauri/target/release/bundle"
)

$ErrorActionPreference = "Stop"

$msi = Get-ChildItem -Path $BundleRoot -Recurse -Filter *.msi | Select-Object -First 1
$exe = Get-ChildItem -Path $BundleRoot -Recurse -Filter *.exe | Select-Object -First 1

function Find-AppExecutable {
  $roots = @(
    "$env:LOCALAPPDATA\Programs",
    "$env:ProgramFiles",
    "$env:ProgramFiles(x86)"
  ) | Where-Object { Test-Path $_ }

  foreach ($root in $roots) {
    $candidate = Get-ChildItem -Path $root -Recurse -Filter class-copilot.exe -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($candidate) {
      return $candidate.FullName
    }
  }

  return $null
}

function Start-And-StopApp {
  param([string]$ExecutablePath)

  $process = Start-Process -FilePath $ExecutablePath -PassThru
  Start-Sleep -Seconds 8
  if ($process.HasExited) {
    throw "Installed app exited unexpectedly: $ExecutablePath"
  }
  Stop-Process -Id $process.Id -Force
}

if (-not $msi -and -not $exe) {
  throw "No Windows installer found under $BundleRoot"
}

if ($msi) {
  Start-Process msiexec.exe -ArgumentList "/i `"$($msi.FullName)`" /qn /norestart" -Wait -NoNewWindow
  $installedExe = Find-AppExecutable
  if (-not $installedExe) {
    throw "Installed app executable not found after MSI install"
  }
  Start-And-StopApp -ExecutablePath $installedExe
  Start-Process msiexec.exe -ArgumentList "/x `"$($msi.FullName)`" /qn /norestart" -Wait -NoNewWindow
}

if ($exe) {
  Start-Process -FilePath $exe.FullName -ArgumentList "/S" -Wait -NoNewWindow
  $installedExe = Find-AppExecutable
  if (-not $installedExe) {
    throw "Installed app executable not found after EXE install"
  }
  Start-And-StopApp -ExecutablePath $installedExe
  $uninstaller = Get-ChildItem -Path (Split-Path $installedExe -Parent) -Filter Uninstall*.exe -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($uninstaller) {
    Start-Process -FilePath $uninstaller.FullName -ArgumentList "/S" -Wait -NoNewWindow
  }
}
