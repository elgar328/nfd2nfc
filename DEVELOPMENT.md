# Development Guide

## Development Setup

Homebrew로 설치된 버전과 개발 빌드는 동일한 plist 경로(`~/Library/LaunchAgents/homebrew.mxcl.nfd2nfc.plist`)를 사용하므로 충돌한다. 개발 시에는 brew 버전을 제거하고 수동으로 plist를 생성해야 한다.

### 개발 환경 전환 (brew → dev)

```bash
# 1. brew 버전 제거 (서비스 중지 + plist 삭제 자동 처리)
brew uninstall nfd2nfc

# 2. 릴리즈 빌드
cargo build --release

# 3. 개발용 plist 생성
cat > ~/Library/LaunchAgents/homebrew.mxcl.nfd2nfc.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>homebrew.mxcl.nfd2nfc</string>
    <key>ProgramArguments</key>
    <array>
        <string>/path/to/nfd2nfc/target/release/nfd2nfc-watcher</string>
    </array>
    <key>KeepAlive</key>
    <dict>
        <key>Crashed</key>
        <true/>
    </dict>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
EOF

# 4. TUI 실행 (TUI에서 watcher start/stop 가능)
cargo run --bin nfd2nfc
```

코드 수정 후: `cargo build --release` → TUI에서 watcher restart

### 개발 환경 정리 (dev → brew)

```bash
# 1. LaunchAgent 해제 + plist 삭제
launchctl unload ~/Library/LaunchAgents/homebrew.mxcl.nfd2nfc.plist
rm ~/Library/LaunchAgents/homebrew.mxcl.nfd2nfc.plist

# 2. brew 버전 재설치
brew install elgar328/nfd2nfc/nfd2nfc
```

## Release Procedure

main 브랜치에서 직접 개발하며, 릴리스 시에만 버전 확정.

```bash
# 1. 버전 확정: Cargo.toml "X.Y.Z-dev" → "X.Y.Z"
cargo check
git add Cargo.toml Cargo.lock
git commit -m "Release vX.Y.Z"
git push origin main

# 2. 태그 푸시 → GitHub Actions 자동 릴리스
git tag vX.Y.Z
git push origin --tags

# 3. 다음 개발 버전 설정
# Cargo.toml: "X.Y.Z" → "X.Y.(Z+1)-dev"
cargo check
git add Cargo.toml Cargo.lock
git commit -m "Bump version to X.Y.(Z+1)-dev"
git push origin main
```

### 자동으로 수행되는 작업 (release.yml)

1. macOS x86_64 + aarch64 빌드
2. `lipo -create`로 universal binary 생성
3. `nfd2nfc-{VERSION}-universal-apple-darwin.tar.gz` 패키징
4. GitHub Release 생성 (release notes 자동 생성, 웹에서 수정 가능)
5. `homebrew-nfd2nfc` tap Formula 자동 업데이트 (URL + SHA256)
