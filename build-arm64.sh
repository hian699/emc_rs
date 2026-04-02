#!/bin/bash

# Build script for ARM64 Docker images
# Requires docker buildx support

set -e

# Configuration
IMAGE_REPOSITORY="${1:-your-dockerhub-username/emc-rs}"
IMAGE_TAG="${2:-latest}"
PUSH="${3:-false}"
IMAGE_NAME="$IMAGE_REPOSITORY:$IMAGE_TAG"
PLATFORMS="linux/amd64,linux/arm64"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔨 Building Docker image for multiple platforms (amd64, arm64)...${NC}"
echo -e "${BLUE}📦 Image: $IMAGE_NAME${NC}"
echo -e "${BLUE}🎯 Platforms: $PLATFORMS${NC}"
echo ""

# Check if docker buildx is available
echo -e "${BLUE}✓ Checking Docker buildx availability...${NC}"
if ! docker buildx version &>/dev/null; then
    echo -e "${RED}❌ Docker buildx is not available. Please install it or enable it in Docker Desktop.${NC}"
    exit 1
fi

# Build command
BUILD_ARGS=(
    "buildx" "build"
    "--platform" "$PLATFORMS"
    "-t" "$IMAGE_NAME"
)

if [ "$PUSH" = "true" ] || [ "$PUSH" = "1" ]; then
    BUILD_ARGS+=("--push")
    echo -e "${YELLOW}📤 Will push image to registry after build${NC}"
else
    echo -e "${YELLOW}📝 Build only (use 'true' as third argument to push to registry)${NC}"
fi

BUILD_ARGS+=(".")

echo ""
echo -e "${BLUE}Running: docker ${BUILD_ARGS[*]}${NC}"
echo ""

# Run build
docker "${BUILD_ARGS[@]}"

if [ $? -eq 0 ]; then
    echo ""
    echo -e "${GREEN}✅ Build successful!${NC}"
    echo ""
    echo -e "${BLUE}Next steps:${NC}"
    if [ "$PUSH" = "true" ] || [ "$PUSH" = "1" ]; then
        echo "• Image has been pushed to $IMAGE_REPOSITORY"
    else
        echo "• To push to registry: docker buildx build --platform $PLATFORMS -t $IMAGE_NAME --push ."
        echo "• Or re-run this script with 'true': ./build-arm64.sh $IMAGE_REPOSITORY $IMAGE_TAG true"
    fi
    echo "• Update docker-compose.yml with: BOT_IMAGE=$IMAGE_NAME"
else
    echo ""
    echo -e "${RED}❌ Build failed!${NC}"
    exit 1
fi
