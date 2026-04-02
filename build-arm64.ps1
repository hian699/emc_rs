# Build script for ARM64 Docker images
# Require docker buildx support

param(
    [string]$ImageRepository = "your-dockerhub-username/emc-rs",
    [string]$ImageTag = "latest",
    [switch]$Push = $false
)

$ImageName = "$ImageRepository`:$ImageTag"
$Platforms = "linux/amd64,linux/arm64"

Write-Host "🔨 Building Docker image for multiple platforms (amd64, arm64)..."
Write-Host "📦 Image: $ImageName"
Write-Host "🎯 Platforms: $Platforms"
Write-Host ""

# Check if docker buildx is available
Write-Host "✓ Checking Docker buildx availability..."
docker buildx version | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Docker buildx is not available. Please install it or enable it in Docker Desktop."
    exit 1
}

# Build command
$BuildArgs = @(
    "buildx", "build",
    "--platform", $Platforms,
    "-t", $ImageName
)

if ($Push) {
    $BuildArgs += "--push"
    Write-Host "📤 Will push image to registry after build"
} else {
    Write-Host "📝 Build only (use -Push flag to push to registry)"
}

$BuildArgs += "."

Write-Host ""
Write-Host "Running: docker $($BuildArgs -join ' ')"
Write-Host ""

# Run build
docker $BuildArgs

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "✅ Build successful!"
    Write-Host ""
    Write-Host "Next steps:"
    if ($Push) {
        Write-Host "• Image has been pushed to $ImageRepository"
    } else {
        Write-Host "• To push to registry: docker buildx build --platform $Platforms -t $ImageName --push ."
        Write-Host "• Or re-run this script with -Push flag: .\build-arm64.ps1 -Push"
    }
    Write-Host "• Update docker-compose.yml with: BOT_IMAGE=$ImageName"
} else {
    Write-Host ""
    Write-Host "❌ Build failed!"
    exit 1
}
