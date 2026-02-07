# xcbridge

Xcode Bridge Service for containerized iOS development.

xcbridge is a lightweight HTTP service that runs on macOS and provides a REST API for Xcode operations. It enables AI agents running in Linux containers to access iOS build tooling that requires macOS and Xcode.

## Why xcbridge?

iOS development requires Xcode, which only runs on macOS. When AI agents run in containerized environments (typically Linux), they cannot directly access Xcode. xcbridge solves this by:

- Running as a service on the macOS host
- Exposing Xcode operations via REST API
- Allowing containers to connect via `http://host.docker.internal:9090`

## Features

- **Build Management**: Trigger builds, stream logs, track status
- **Test Execution**: Run tests with real-time log streaming
- **Simulator Control**: Boot, shutdown, install apps, launch
- **Device Management**: List and interact with physical iOS devices
- **Authentication**: Optional API key protection
- **Streaming Logs**: Server-Sent Events (SSE) for real-time output

## Installation

### Via npm (Recommended)

```bash
# Install globally
npm install -g @aptove/xcbridge

# Or run directly with npx
npx @aptove/xcbridge --port 9090
```

### Prerequisites

- macOS 13 or later
- Xcode 15 or later (with command line tools)

### Build from Source

Requires Rust 1.75+

```bash
# Clone the repository
git clone https://github.com/Aptove/xcbridge.git
cd xcbridge

# Build release binary
cargo build --release

# Install as launchd service
./scripts/install.sh
```

### Install Options

```bash
# Custom port
./scripts/install.sh --port 8080

# With API key authentication
./scripts/install.sh --api-key my-secret-key

# Both
./scripts/install.sh --port 8080 --api-key my-secret-key
```

### Uninstall

```bash
./scripts/uninstall.sh
```

## Usage

### Running Manually

```bash
# Default settings (port 9090)
xcbridge

# Custom port
xcbridge --port 8080

# With API key
xcbridge --api-key your-secret-key

# Verbose logging
xcbridge --log-level debug
```

### From a Container

```bash
# Check status
curl http://host.docker.internal:9090/status

# Start a build
curl -X POST http://host.docker.internal:9090/build \
  -H "Content-Type: application/json" \
  -d '{"project_path": "/path/to/project", "scheme": "MyApp"}'

# Stream build logs
curl http://host.docker.internal:9090/build/{build_id}/logs
```

## API Reference

### Status

```
GET /status
```

Returns service status and Xcode version.

**Response:**
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "xcode_version": "15.2"
}
```

### Build

#### Create Build

```
POST /build
```

**Request Body:**
```json
{
  "project_path": "/path/to/MyApp.xcodeproj",
  "scheme": "MyApp",
  "configuration": "Debug",
  "destination": "platform=iOS Simulator,name=iPhone 15",
  "derived_data_path": "/tmp/DerivedData"
}
```

**Response:**
```json
{
  "build_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "running",
  "started_at": "2024-01-15T10:30:00Z"
}
```

#### Get Build Status

```
GET /build/{build_id}
```

#### Stream Build Logs (SSE)

```
GET /build/{build_id}/logs
```

#### Cancel Build

```
DELETE /build/{build_id}
```

### Test

#### Run Tests

```
POST /test
```

**Request Body:**
```json
{
  "project_path": "/path/to/MyApp.xcodeproj",
  "scheme": "MyAppTests",
  "destination": "platform=iOS Simulator,name=iPhone 15"
}
```

#### Get Test Status

```
GET /test/{test_id}
```

#### Stream Test Logs

```
GET /test/{test_id}/logs
```

### Simulator

#### List Simulators

```
GET /simulator/list
```

**Response:**
```json
{
  "simulators": [
    {
      "udid": "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE",
      "name": "iPhone 15",
      "state": "Shutdown",
      "runtime": "iOS 17.2"
    }
  ]
}
```

#### Boot Simulator

```
POST /simulator/boot
```

**Request Body:**
```json
{
  "udid": "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE"
}
```

#### Shutdown Simulator

```
POST /simulator/shutdown
```

#### Install App

```
POST /simulator/install
```

**Request Body:**
```json
{
  "udid": "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE",
  "app_path": "/path/to/MyApp.app"
}
```

#### Launch App

```
POST /simulator/launch
```

**Request Body:**
```json
{
  "udid": "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE",
  "bundle_id": "com.example.MyApp"
}
```

#### Uninstall App

```
POST /simulator/uninstall
```

### Device (Physical Devices)

#### List Devices

```
GET /device/list
```

#### Install App

```
POST /device/install
```

#### Launch App

```
POST /device/launch
```

#### Uninstall App

```
POST /device/uninstall
```

## Authentication

When running with `--api-key`, all requests must include the `X-API-Key` header:

```bash
curl -H "X-API-Key: your-secret-key" http://localhost:9090/status
```

## Configuration

| Option | Environment Variable | Default | Description |
|--------|---------------------|---------|-------------|
| `--port` | `XCBRIDGE_PORT` | 9090 | Port to listen on |
| `--host` | `XCBRIDGE_HOST` | 127.0.0.1 | Host to bind to |
| `--api-key` | `XCBRIDGE_API_KEY` | - | API key for authentication |
| `--log-level` | `XCBRIDGE_LOG_LEVEL` | info | Log level (trace, debug, info, warn, error) |

## Docker Integration

When using Docker, access xcbridge from containers using `host.docker.internal`:

```yaml
# docker-compose.yml
services:
  agent:
    image: swift:5.9-jammy
    environment:
      XCBRIDGE_URL: http://host.docker.internal:9090
    extra_hosts:
      - "host.docker.internal:host-gateway"
```

## Security Considerations

1. **Network Binding**: By default, xcbridge binds to `127.0.0.1` (localhost only)
2. **API Key**: Use `--api-key` in production environments
3. **Path Restrictions**: Consider using `--allowed-paths` to restrict file system access

## Troubleshooting

### Service won't start

Check logs:
```bash
tail -f ~/Library/Logs/xcbridge.log
tail -f ~/Library/Logs/xcbridge.error.log
```

### Xcode not found

Ensure Xcode is installed and command line tools are configured:
```bash
xcode-select -p
xcodebuild -version
```

### Container can't connect

Verify the service is running:
```bash
curl http://localhost:9090/status
```

Check Docker networking:
```bash
# From inside container
curl http://host.docker.internal:9090/status
```

## Development

### Running Tests

```bash
cargo test
```

### Building for Release

```bash
cargo build --release
```

The binary will be at `target/release/xcbridge`.

## License

Apache License 2.0. See [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting PRs.
