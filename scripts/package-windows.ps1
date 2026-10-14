# Makes the Windows release: dist\FretStudy-<version>-setup.exe.
# Run on a Windows PC with Rust (https://rustup.rs) and Inno Setup 6 installed.
#
# Optional code signing (removes the "unknown publisher" SmartScreen warning):
#   $env:FRETSTUDY_SIGN_CERT = "C:\path\to\certificate.pfx"
#   $env:FRETSTUDY_SIGN_PASSWORD = "..."
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

cargo build --release -p fretstudy
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

$version = (Select-String -Path crates\app\Cargo.toml -Pattern '^version = "(.*)"').Matches[0].Groups[1].Value
$env:FRETSTUDY_VERSION = $version

function Sign($file) {
    if ($env:FRETSTUDY_SIGN_CERT) {
        signtool sign /f $env:FRETSTUDY_SIGN_CERT /p $env:FRETSTUDY_SIGN_PASSWORD `
            /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 $file
        if ($LASTEXITCODE -ne 0) { throw "signing $file failed" }
    }
}

Sign "target\release\fretstudy.exe"
$iscc = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
& $iscc installer\fretstudy.iss
if ($LASTEXITCODE -ne 0) { throw "Inno Setup failed" }
Sign "dist\FretStudy-$version-setup.exe"
Write-Host "Built dist\FretStudy-$version-setup.exe"
