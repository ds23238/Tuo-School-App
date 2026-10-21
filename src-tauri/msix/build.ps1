# Packt den Tauri-Release-Build als MSIX und signiert ihn.
# Zertifikat: Secret MSIX_CERT_PFX (Base64) + MSIX_CERT_PASSWORD, sonst ein
# temporäres selbst signiertes Zertifikat (muss dann pro Release neu importiert werden).
$ErrorActionPreference = "Stop"

$root = Resolve-Path "$PSScriptRoot/../.."
$publisher = "CN=ds23238"
$out = Join-Path $root "target/release/bundle/msix"
$stage = Join-Path $out "stage"

$meta = cargo metadata --no-deps --format-version 1 --manifest-path "$root/Cargo.toml" | ConvertFrom-Json
$version = ($meta.packages | Where-Object name -eq "tsa-sync").version
$msixVersion = ($version -replace "[-+].*$", "") + ".0"

# Neuestes SDK, das makeappx enthält; nicht jeder SDK-Ordner bringt es mit.
$makeappx = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\makeappx.exe" |
    Sort-Object { [version]$_.Directory.Parent.Name } -Descending | Select-Object -First 1
if (-not $makeappx) { throw "makeappx.exe nicht gefunden" }
$signtool = Join-Path $makeappx.DirectoryName "signtool.exe"
$makeappx = $makeappx.FullName

Remove-Item $out -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory "$stage/Assets" | Out-Null
Copy-Item "$root/target/release/tsa-sync.exe" $stage
foreach ($icon in "StoreLogo", "Square150x150Logo", "Square44x44Logo") {
    Copy-Item "$root/src-tauri/icons/$icon.png" "$stage/Assets/"
}
(Get-Content "$PSScriptRoot/AppxManifest.xml" -Raw).
    Replace("{PUBLISHER}", $publisher).
    Replace("{VERSION}", $msixVersion) |
    Set-Content "$stage/AppxManifest.xml" -Encoding utf8

$msix = Join-Path $out "TSA-Sync_${version}_x64.msix"
& $makeappx pack /o /d $stage /p $msix
if ($LASTEXITCODE) { throw "makeappx fehlgeschlagen" }

$pfx = Join-Path $out "cert.pfx"
if ($env:MSIX_CERT_PFX) {
    [IO.File]::WriteAllBytes($pfx, [Convert]::FromBase64String($env:MSIX_CERT_PFX))
    $password = $env:MSIX_CERT_PASSWORD
    $cert = New-Object Security.Cryptography.X509Certificates.X509Certificate2($pfx, $password)
} else {
    Write-Warning "Kein MSIX_CERT_PFX gesetzt, verwende temporäres Zertifikat."
    $password = [guid]::NewGuid().ToString()
    $cert = New-SelfSignedCertificate -Type Custom -Subject $publisher -KeyUsage DigitalSignature `
        -FriendlyName "TSA-Sync" -CertStoreLocation "Cert:\CurrentUser\My" `
        -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")
    Export-PfxCertificate -Cert $cert -FilePath $pfx -Password (ConvertTo-SecureString $password -AsPlainText -Force) | Out-Null
}
if ($cert.Subject -ne $publisher) { throw "Zertifikat-Subject '$($cert.Subject)' passt nicht zu '$publisher'" }

& $signtool sign /fd SHA256 /f $pfx /p $password $msix
if ($LASTEXITCODE) { throw "signtool fehlgeschlagen" }

[IO.File]::WriteAllBytes((Join-Path $out "TSA-Sync.cer"), $cert.Export("Cert"))
Copy-Item "$PSScriptRoot/install-cert.ps1" $out
Remove-Item $stage, $pfx -Recurse -Force
