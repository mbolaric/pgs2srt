# Packages target\release into a portable zip and an Inno Setup installer. The executables link Tesseract,
# Leptonica and the C runtime statically (vcpkg *-windows-static* triplet + crt-static), so only data and
# license files are added next to them.
# Usage: build-installer.ps1 -Version X.Y.Z -Common <dir> -Out <dir>
#   Common holds tessdata\, licenses\ and THIRD-PARTY-LICENSES.html from the release prepare job.
#   VCPKG_ROOT and VCPKGRS_TRIPLET must point at the vcpkg tree the executables were linked against.
param(
    [Parameter(Mandatory)] [string] $Version,
    [Parameter(Mandatory)] [string] $Common,
    [Parameter(Mandatory)] [string] $Out
)
$ErrorActionPreference = 'Stop'

$root = Resolve-Path "$PSScriptRoot\..\.."
$stage = Join-Path $root 'target\windows\pgs2srt'
New-Item -ItemType Directory -Force $Out | Out-Null
$Out = Resolve-Path $Out
Remove-Item -Recurse -Force (Split-Path $stage) -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $stage | Out-Null

Copy-Item "$root\target\release\pgs2srt-desktop.exe", "$root\target\release\pgs2srt.exe", "$root\packaging\icons\pgs2srt.ico" $stage
Copy-Item "$root\LICENSE" "$stage\LICENSE.txt"
Copy-Item "$Common\THIRD-PARTY-LICENSES.html" $stage
# languages::data_path() looks for tessdata next to the executable.
Copy-Item -Recurse "$Common\tessdata", "$Common\licenses" $stage

# Every vcpkg port linked into the executables ships its license as share\<port>\copyright.
$share = Join-Path $env:VCPKG_ROOT "installed\$env:VCPKGRS_TRIPLET\share"
foreach ($copyright in Get-ChildItem "$share\*\copyright") {
    Copy-Item $copyright.FullName "$stage\licenses\$($copyright.Directory.Name).txt"
}

# The executables must only import DLLs that ship with Windows.
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$dumpbin = & $vswhere -latest -products * -find 'VC\Tools\MSVC\*\bin\Hostx64\x64\dumpbin.exe' | Select-Object -First 1
foreach ($exe in 'pgs2srt-desktop.exe', 'pgs2srt.exe') {
    $imports = & $dumpbin /nologo /dependents "$stage\$exe" | Where-Object { $_ -match '^\s+\S+\.dll$' } | ForEach-Object { $_.Trim() }
    Write-Host "$exe imports: $($imports -join ', ')"
    $bundled = $imports | Where-Object { $_ -match 'vcruntime|msvcp|tesseract|lept|curl|archive|zlib|png|jpeg|tiff|webp|gif|openjp' }
    if ($bundled) { throw "$exe depends on non-system DLLs: $($bundled -join ', ')" }
}
& "$stage\pgs2srt.exe" --version
if ($LASTEXITCODE -ne 0) { throw 'pgs2srt.exe --version failed' }

Compress-Archive -Path $stage -DestinationPath "$Out\pgs2srt-$Version-windows-x64-portable.zip" -Force

$iscc = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
if (-not (Test-Path $iscc)) {
    choco install innosetup --no-progress -y
    if ($LASTEXITCODE -ne 0) { throw 'Installing Inno Setup failed' }
}
& $iscc /Qp "/DAppVersion=$Version" "/DSourceDir=$stage" "/DOutputDir=$Out" "$root\packaging\windows\pgs2srt.iss"
if ($LASTEXITCODE -ne 0) { throw 'ISCC failed' }
