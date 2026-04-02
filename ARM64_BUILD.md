# Build ARM64 (aarch64) Support

Hướng dẫn cập nhật bot Discord để chạy trên thiết bị ARM64 (Raspberry Pi, Orange Pi, thiết bị embedding khác).

## Yêu cầu

### Hệ điều hành Windows (PowerShell)
- Docker Desktop with buildx support
- PowerShell 5.0 hoặc cao hơn

### Linux/macOS
- Docker với buildx enabled
- Bash shell

## Thiết lập Docker Buildx

### Windows
```powershell
# Kiểm tra buildx
docker buildx version

# Nếu chưa có, tạo builder mới
docker buildx create --name multiarch-builder
docker buildx use multiarch-builder
docker buildx inspect --bootstrap
```

### Linux
```bash
# Kiểm tra buildx
docker buildx version

# Nếu chưa có, cài đặt buildx
mkdir -p ~/.docker/cli-plugins
wget https://github.com/docker/buildx/releases/download/v0.14.0/buildx-v0.14.0.linux-amd64 -O ~/.docker/cli-plugins/docker-buildx
chmod +x ~/.docker/cli-plugins/docker-buildx

# Tạo builder mới
docker buildx create --name multiarch-builder
docker buildx use multiarch-builder
docker buildx inspect --bootstrap
```

## Build cho ARM64

### Cách 1: Sử dụng Script (Khuyến nghị)

#### Windows (PowerShell)
```powershell
# Chỉ build (không push)
.\build-arm64.ps1 -ImageRepository "your-dockerhub-username/emc-rs" -ImageTag "latest"

# Build và push lên Docker Hub
.\build-arm64.ps1 -ImageRepository "your-dockerhub-username/emc-rs" -ImageTag "latest" -Push
```

#### Linux/macOS
```bash
# Chỉ build (không push)
./build-arm64.sh your-dockerhub-username/emc-rs latest

# Build và push lên Docker Hub
./build-arm64.sh your-dockerhub-username/emc-rs latest true
```

### Cách 2: Build Manual

#### Chỉ build cho ARM64
```bash
docker buildx build --platform linux/arm64 -t your-dockerhub-username/emc-rs:latest --load .
```

#### Build cho cả amd64 và arm64
```bash
docker buildx build --platform linux/amd64,linux/arm64 -t your-dockerhub-username/emc-rs:latest .
```

#### Build và push lên Docker Hub
```bash
docker buildx build --platform linux/amd64,linux/arm64 -t your-dockerhub-username/emc-rs:latest --push .
```

## File thay đổi

### 1. docker-compose.yml
- ✅ Thêm `platform: linux/arm64` cho service `bot`
- ✅ Thêm `platform: linux/arm64` cho service `lavalink`
- ✅ Thêm `platform: linux/arm64` cho service `fix-perms`

### 2. Dockerfile
- ✅ Thêm `ARG BUILDPLATFORM`, `ARG TARGETPLATFORM`, `ARG TARGETARCH` cho flexibility
- ✅ Sử dụng `--platform=$BUILDPLATFORM` trong build stage
- ✅ Sử dụng `--platform=$TARGETPLATFORM` trong runtime stage

## Deployment

### Cập nhật environment variable
```bash
# Nếu build và push lên Docker Hub
export BOT_IMAGE="your-dockerhub-username/emc-rs:latest"

# Hoặc set trong .env
BOT_IMAGE=your-dockerhub-username/emc-rs:latest
```

### Deploy trên ARM device
```bash
docker-compose up -d
```

## Troubleshooting

### Error: "docker buildx" not found
```bash
# Cài đặt buildx
docker buildx version
```

### Error: "no matching manifest" 
Điều này có nghĩa là image không hỗ trợ ARM64. Kiểm tra:
- Lavalink image `ghcr.io/lavalink-devs/lavalink:4.2` hỗ trợ arm64
- Base images `rust:1.88-slim` và `debian:bookworm-slim` hỗ trợ arm64

### Build chậm
- Build cho ARM64 có thể chậm hơn, đặc biệt là compile Rust
- Có thể setup caching để tăng tốc độ

### Memory issues
- Nếu gặp lỗi memory, tăng Java memory trong docker-compose.yml:
```yaml
environment:
  _JAVA_OPTIONS: "-Xmx2G"  # Giảm từ 6G để phù hợp với ARM device
```

## Resources

- [Docker Buildx Documentation](https://docs.docker.com/build/buildx/)
- [Multi-platform builds](https://docs.docker.com/build/building/multi-platform/)
- [Lavalink Releases](https://github.com/lavalink-devs/Lavalink/releases)
