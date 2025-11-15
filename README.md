# Datadog CLI

빠르고 강력한 Datadog API 조회 도구 - 자연어 시간 파싱 지원

## ✨ 주요 기능

- **⚡ 고성능**: Rust 기반, Python SDK 대비 10배 빠른 조회 속도
- **🔒 안전한 인증**: rustls 기반 TLS 1.3 보안 통신
- **📊 다양한 출력**: JSON, JSONL, Table 지원으로 Unix 파이프라인 연동
- **🕐 자연어 시간**: "1 hour ago", "30 minutes ago" 등 직관적인 시간 지정
- **⚙️ 유연한 설정**: CLI 인자, 환경 변수, 프로젝트/전역 설정 파일 지원

## 🚀 빠른 시작

### 설치

```bash
# Cargo로 설치
cargo install --path .

# 또는 스크립트 사용
./scripts/install.sh
```

### 설정

```bash
# 1. 설정 파일 생성
datadog config init

# 2. API 키 설정 (3가지 방법 중 택1)
export DD_API_KEY="your-api-key"
export DD_APP_KEY="your-app-key"

# 또는
datadog config edit

# 또는
datadog --api-key "key" --app-key "key" metrics "..."
```

### 기본 사용

```bash
# 메트릭 조회 (최근 1시간)
datadog metrics "system.cpu.user"

# 로그 검색
datadog logs search "service:web status:error" --from "1 hour ago"

# 모니터 목록
datadog monitors list
```

## 📖 주요 명령어

| 명령어 | 설명 | 예시 |
|--------|------|------|
| `metrics` | 메트릭 조회 | `datadog metrics "avg:system.cpu.user{*}"` |
| `logs` | 로그 검색/분석 | `datadog logs search "query" --limit 100` |
| `monitors` | 모니터 관리 | `datadog monitors list --tags "env:prod"` |
| `events` | 이벤트 조회 | `datadog events --from "1 day ago"` |
| `hosts` | 호스트 목록 | `datadog hosts --filter "env:production"` |
| `dashboards` | 대시보드 관리 | `datadog dashboards list` |
| `spans` | APM 스팬 검색 | `datadog spans "service:api" --from "..." --to "..."` |
| `services` | 서비스 목록 | `datadog services --env prod` |
| `rum` | RUM 이벤트 검색 | `datadog rum "query"` |
| `config` | 설정 관리 | `datadog config show` |

## ⚙️ 설정

### 우선순위

```
1. CLI 인자          --api-key, --app-key (최우선)
2. 환경 변수         DD_API_KEY, DD_APP_KEY, DD_SITE
3. 프로젝트 설정     ./.datadog.toml
4. 전역 설정         ~/.config/datadog-cli/config.toml
```

### 설정 파일 예시

**전역 설정** (`~/.config/datadog-cli/config.toml`):

```toml
api_key = "your-api-key-here"
app_key = "your-app-key-here"
site = "datadoghq.com"  # 또는 datadoghq.eu, ddog-gov.com 등
```

**프로젝트 설정** (`.datadog.toml`):

```toml
# 프로젝트별 다른 키 사용
api_key = "project-specific-key"
app_key = "project-specific-app-key"
site = "datadoghq.eu"
```

### 환경 변수

```bash
export DD_API_KEY="your-api-key"
export DD_APP_KEY="your-app-key"
export DD_SITE="datadoghq.com"
```

## 💡 유용한 팁

### jq와 함께 사용

```bash
# 메트릭 포인트만 추출
datadog metrics "system.cpu.user" --format jsonl | jq '.series[].pointlist'

# 로그 메시지만 추출
datadog logs search "query" --format jsonl | jq -r '.logs[].message'
```

### 시간 파싱

```bash
# 자연어
datadog metrics "..." --from "1 hour ago" --to "now"
datadog logs search "..." --from "30 minutes ago"

# ISO8601
datadog metrics "..." --from "2024-01-01T00:00:00Z" --to "2024-01-01T23:59:59Z"

# Unix timestamp
datadog metrics "..." --from "1704067200" --to "1704153600"
```

### Table 출력

```bash
# 읽기 쉬운 테이블 형식
datadog monitors list --format table
datadog hosts --format table
```

## 🛠️ 문제 해결

### 설정 파일을 찾을 수 없음

**증상**: `Config not found` 에러

**해결**:
```bash
# 1. 설정 파일 생성
datadog config init

# 2. 설정 파일 경로 확인
datadog config path

# 3. API 키 설정
datadog config edit
```

### 인증 실패

**증상**: `AuthError` 또는 403 에러

**해결**:
1. API 키 확인: `datadog config show`
2. Datadog에서 API 키 재생성
3. 환경 변수로 테스트:
   ```bash
   DD_API_KEY="new-key" DD_APP_KEY="new-app-key" datadog monitors list
   ```

### 잘못된 Site

**증상**: `Invalid site` 에러

**해결**:
```bash
# Site 확인 및 수정
datadog config edit
# site를 다음 중 하나로 설정:
# - datadoghq.com (US1)
# - datadoghq.eu (EU)
# - ddog-gov.com (US1-FED)
# - us3.datadoghq.com (US3)
# - us5.datadoghq.com (US5)
# - ap1.datadoghq.com (AP1)
```

## 🔧 개발

### 요구사항

- Rust 1.91.1 이상
- Cargo

### 빌드

```bash
# 개발 빌드
cargo build

# 릴리즈 빌드 (최적화)
cargo build --release

# 실행
cargo run -- metrics "system.cpu.user"
```

### 테스트

```bash
# 모든 테스트
cargo test

# 특정 테스트
cargo test test_name

# 디버그 로그와 함께
RUST_LOG=debug cargo test
```

### 코드 품질

```bash
# Lint
cargo clippy -- -D warnings

# 포맷
cargo fmt

# 모두 실행
cargo fmt && cargo clippy -- -D warnings && cargo test
```

## 🤝 기여

이슈와 PR을 환영합니다!

1. Fork
2. Feature 브랜치 생성 (`git checkout -b feature/amazing-feature`)
3. Commit (`git commit -m 'Add amazing feature'`)
4. Push (`git push origin feature/amazing-feature`)
5. Pull Request

## 📄 라이선스

MIT License - [LICENSE](LICENSE) 참고

## 🔗 링크

- [Datadog API 문서](https://docs.datadoghq.com/api/)
- [GitHub Repository](https://github.com/junyeong-ai/datadog-cli)
- [Issue Tracker](https://github.com/junyeong-ai/datadog-cli/issues)
