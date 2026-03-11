# Helper script for local Windows development.
# Adds MSVC nmake to PATH (required by audiopus_sys when building songbird)
# then runs cargo check with lavalink feature.

$msvcBin = "D:\IDE\vstim26\VC\Tools\MSVC\14.44.35207\bin\HostX64\x64"
if (Test-Path $msvcBin) {
    $env:PATH = "$msvcBin;$env:PATH"
}

cargo check --features lavalink @args
