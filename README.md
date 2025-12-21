# Datadog CLI

[![CI](https://github.com/junyeong-ai/datadog-cli/workflows/CI/badge.svg)](https://github.com/junyeong-ai/datadog-cli/actions)
[![Lint](https://github.com/junyeong-ai/datadog-cli/workflows/Lint/badge.svg)](https://github.com/junyeong-ai/datadog-cli/actions)
[![codecov](https://codecov.io/gh/junyeong-ai/datadog-cli/branch/main/graph/badge.svg)](https://codecov.io/gh/junyeong-ai/datadog-cli)
[![Rust](https://img.shields.io/badge/rust-1.91.1%2B%20(2024%20edition)-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/version-0.1.0-blue?style=flat-square)](https://github.com/junyeong-ai/datadog-cli/releases)
[![DeepWiki](https://img.shields.io/badge/DeepWiki-junyeong--ai%2Fdatadog--cli-blue.svg?logo=data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACwAAAAyCAYAAAAnWDnqAAAAAXNSR0IArs4c6QAAA05JREFUaEPtmUtyEzEQhtWTQyQLHNak2AB7ZnyXZMEjXMGeK/AIi+QuHrMnbChYY7MIh8g01fJoopFb0uhhEqqcbWTp06/uv1saEDv4O3n3dV60RfP947Mm9/SQc0ICFQgzfc4CYZoTPAswgSJCCUJUnAAoRHOAUOcATwbmVLWdGoH//PB8mnKqScAhsD0kYP3j/Yt5LPQe2KvcXmGvRHcDnpxfL2zOYJ1mFwrryWTz0advv1Ut4CJgf5uhDuDj5eUcAUoahrdY/56ebRWeraTjMt/00Sh3UDtjgHtQNHwcRGOC98BJEAEymycmYcWwOprTgcB6VZ5JK5TAJ+fXGLBm3FDAmn6oPPjR4rKCAoJCal2eAiQp2x0vxTPB3ALO2CRkwmDy5WohzBDwSEFKRwPbknEggCPB/imwrycgxX2NzoMCHhPkDwqYMr9tRcP5qNrMZHkVnOjRMWwLCcr8ohBVb1OMjxLwGCvjTikrsBOiA6fNyCrm8V1rP93iVPpwaE+gO0SsWmPiXB+jikdf6SizrT5qKasx5j8ABbHpFTx+vFXp9EnYQmLx02h1QTTrl6eDqxLnGjporxl3NL3agEvXdT0WmEost648sQOYAeJS9Q7bfUVoMGnjo4AZdUMQku50McDcMWcBPvr0SzbTAFDfvJqwLzgxwATnCgnp4wDl6Aa+Ax283gghmj+vj7feE2KBBRMW3FzOpLOADl0Isb5587h/U4gGvkt5v60Z1VLG8BhYjbzRwyQZemwAd6cCR5/XFWLYZRIMpX39AR0tjaGGiGzLVyhse5C9RKC6ai42ppWPKiBagOvaYk8lO7DajerabOZP46Lby5wKjw1HCRx7p9sVMOWGzb/vA1hwiWc6jm3MvQDTogQkiqIhJV0nBQBTU+3okKCFDy9WwferkHjtxib7t3xIUQtHxnIwtx4mpg26/HfwVNVDb4oI9RHmx5WGelRVlrtiw43zboCLaxv46AZeB3IlTkwouebTr1y2NjSpHz68WNFjHvupy3q8TFn3Hos2IAk4Ju5dCo8B3wP7VPr/FGaKiG+T+v+TQqIrOqMTL1VdWV1DdmcbO8KXBz6esmYWYKPwDL5b5FA1a0hwapHiom0r/cKaoqr+27/XcrS5UwSMbQAAAABJRU5ErkJggg==)](https://deepwiki.com/junyeong-ai/datadog-cli)

> **🌐 한국어** | **[English](README.en.md)**

---

> **⚡ 빠르고 강력한 Datadog API 조회 도구**
>
> - 🚀 **고성능** (Rust 기반, Python SDK 대비 10배 빠름)
> - 🕐 **자연어 시간** ("1 hour ago", "30 minutes ago")
> - 📊 **다양한 출력** (JSON, JSONL, Table)
> - 🔒 **안전한 인증** (rustls 기반 TLS 1.3)

---

## ⚡ 빠른 시작 (1분)

```bash
# 1. 설치
curl -fsSL https://raw.githubusercontent.com/junyeong-ai/datadog-cli/main/scripts/install.sh | bash

# 2. 설정 초기화
datadog-cli config init

# 3. API 키 설정
datadog-cli config edit

# 4. 사용 시작! 🎉
datadog-cli monitors list
datadog-cli logs search "status:error" --from "1 hour ago"
datadog-cli metrics "avg:system.cpu.user{*}"
```

---

## 🎯 주요 기능

### 로그 조회
```bash
# 로그 검색 (자연어 시간)
datadog-cli logs search "service:web status:error" --from "1 hour ago"

# 로그 집계 (카운트)
datadog-cli logs aggregate "service:api" --from "6 hours ago"

# 시계열 분석
datadog-cli logs timeseries "status:error" \
  --from "24 hours ago" \
  --interval "1h" \
  --aggregation "count"
```

### 메트릭 조회
```bash
# 메트릭 쿼리
datadog-cli metrics "avg:system.cpu.user{*}"

# 특정 태그 필터링
datadog-cli metrics "avg:system.cpu.user{service:web}"

# 그룹화
datadog-cli metrics "avg:system.cpu.user{*} by {service}"
```

### APM & RUM
```bash
# 스팬 검색 (에러만)
datadog-cli spans "service:api error:true" --from "30 minutes ago"

# RUM 이벤트
datadog-cli rum "@type:error" --from "1 hour ago"

# 서비스 목록
datadog-cli services --env production
```

### 모니터링
```bash
# 모니터 목록
datadog-cli monitors list --tags "env:prod"

# 모니터 상세 조회
datadog-cli monitors get 12345678

# 이벤트 조회
datadog-cli events --from "1 day ago" --priority "normal"
```

### 인프라
```bash
# 호스트 목록
datadog-cli hosts --filter "env:production"

# 대시보드 목록
datadog-cli dashboards list
```

---

## 📦 설치

### 방법 1: Prebuilt Binary (권장) ⭐

**자동 설치**:
```bash
curl -fsSL https://raw.githubusercontent.com/junyeong-ai/datadog-cli/main/scripts/install.sh | bash
```

**수동 설치**:
1. [Releases](https://github.com/junyeong-ai/datadog-cli/releases)에서 바이너리 다운로드
2. 압축 해제: `tar -xzf datadog-*.tar.gz`
3. PATH에 이동: `mv datadog-cli ~/.local/bin/`

### 방법 2: Cargo

```bash
cargo install datadog-cli
```

### 방법 3: 소스 빌드

```bash
git clone https://github.com/junyeong-ai/datadog-cli
cd datadog-cli
./scripts/install.sh
```

**Requirements**: Rust 1.91.1+

### 🤖 Claude Code Skill (선택사항)

`./scripts/install.sh` 실행 시 Claude Code 스킬 설치 여부를 선택할 수 있습니다:

- **User-level** (권장): 모든 프로젝트에서 사용 가능
- **Project-level**: Git을 통해 팀 자동 배포
- **Skip**: 나중에 수동 설치

스킬을 설치하면 Claude Code에서 자연어로 Datadog 데이터 조회가 가능합니다.

---

## ⚙️ 설정

### 우선순위

```
1. CLI 인자          --api-key, --app-key (최우선)
2. 환경 변수         DD_API_KEY, DD_APP_KEY, DD_SITE
3. 프로젝트 설정     ./.datadog.toml
4. 전역 설정         ~/.config/datadog-cli/config.toml
```

### 설정 파일

**전역 설정** (`~/.config/datadog-cli/config.toml`):

```toml
api_key = "your-api-key-here"
app_key = "your-app-key-here"
site = "datadoghq.com"  # 또는 datadoghq.eu, ddog-gov.com 등

[defaults]
format = "json"           # 출력 형식: json, jsonl, table
time_range = "1 hour ago" # 기본 시간 범위
limit = 10                # 기본 결과 수
page_size = 100           # 페이지당 항목 수
# tag_filter = "env:,service:"  # 태그 필터 (선택)

[network]
timeout_secs = 30         # 요청 타임아웃 (초)
max_retries = 3           # 최대 재시도 횟수
```

**프로젝트 설정** (`.datadog.toml`):

```toml
# 프로젝트별 다른 키 사용
api_key = "project-specific-key"
app_key = "project-specific-app-key"
site = "datadoghq.eu"
```

### 설정 관리

```bash
# 설정 초기화
datadog-cli config init

# 설정 표시 (토큰 마스킹)
datadog-cli config show

# 설정 파일 경로
datadog-cli config path

# 에디터로 수정 ($EDITOR 사용)
datadog-cli config edit
```

### 환경 변수

```bash
export DD_API_KEY="your-api-key"
export DD_APP_KEY="your-app-key"
export DD_SITE="datadoghq.com"
```

---

## 💡 사용 팁

### 자연어 시간 파싱

```bash
# 자연어 (권장)
datadog-cli logs search "query" --from "1 hour ago" --to "now"
datadog-cli metrics "query" --from "30 minutes ago"

# ISO8601
datadog-cli logs search "query" --from "2024-01-01T00:00:00Z"

# Unix timestamp
datadog-cli metrics "query" --from "1704067200"
```

### Unix 파이프라인 연동

```bash
# jq로 메트릭 포인트 추출
datadog-cli metrics "system.cpu.user" --format jsonl | jq '.series[].pointlist'

# 로그 메시지만 추출
datadog-cli logs search "query" --format jsonl | jq -r '.logs[].message'

# 에러 카운트
datadog-cli logs search "status:error" --format jsonl | jq '.logs | length'
```

### Table 출력

```bash
# 읽기 쉬운 테이블 형식
datadog-cli monitors list --format table
datadog-cli hosts --format table
```

### 태그 필터링

```bash
# 응답 크기 30-70% 절감
datadog-cli logs search "query" --tag-filter "env:,service:"

# 모든 태그 제외
datadog-cli logs search "query" --tag-filter ""

# 모든 태그 포함 (기본값)
datadog-cli logs search "query" --tag-filter "*"
```

**환경 변수 설정**:
```bash
export DD_TAG_FILTER="env:,service:"
```

**적용 대상**: logs search, spans, rum, hosts

---

## 📖 명령어

| 명령어 | 설명 | 예시 |
|--------|------|------|
| `metrics` | 메트릭 조회 | `datadog-cli metrics "avg:system.cpu.user{*}"` |
| `logs search` | 로그 검색 | `datadog-cli logs search "query" --from "1h ago"` |
| `logs aggregate` | 로그 집계 | `datadog-cli logs aggregate "query" --from "6h ago"` |
| `logs timeseries` | 로그 시계열 | `datadog-cli logs timeseries "query" --interval "1h"` |
| `monitors list` | 모니터 목록 | `datadog-cli monitors list --tags "env:prod"` |
| `monitors get` | 모니터 상세 | `datadog-cli monitors get 12345678` |
| `events` | 이벤트 조회 | `datadog-cli events --from "1 day ago"` |
| `hosts` | 호스트 목록 | `datadog-cli hosts --filter "env:production"` |
| `dashboards list` | 대시보드 목록 | `datadog-cli dashboards list` |
| `dashboards get` | 대시보드 상세 | `datadog-cli dashboards get abc-def-ghi` |
| `spans` | APM 스팬 검색 | `datadog-cli spans "service:api" --from "..."` |
| `services` | 서비스 목록 | `datadog-cli services --env prod` |
| `rum` | RUM 이벤트 검색 | `datadog-cli rum "@type:error"` |
| `config` | 설정 관리 | `datadog-cli config show` |

---

## 🛠️ 문제 해결

### 설정 파일을 찾을 수 없음

**증상**: `Config not found` 에러

**해결**:
```bash
# 1. 설정 파일 생성
datadog-cli config init

# 2. 설정 파일 경로 확인
datadog-cli config path

# 3. API 키 설정
datadog-cli config edit
```

### 인증 실패

**증상**: `AuthError` 또는 403 에러

**해결**:
1. API 키 확인: `datadog-cli config show`
2. Datadog에서 API 키 재생성
3. 환경 변수로 테스트:
   ```bash
   DD_API_KEY="new-key" DD_APP_KEY="new-app-key" datadog-cli monitors list
   ```

### 잘못된 Site

**증상**: `Invalid site` 에러

**해결**:
```bash
# Site 확인 및 수정
datadog-cli config edit
# site를 다음 중 하나로 설정:
# - datadoghq.com (US1)
# - datadoghq.eu (EU)
# - ddog-gov.com (US1-FED)
# - us3.datadoghq.com (US3)
# - us5.datadoghq.com (US5)
# - ap1.datadoghq.com (AP1)
```

---

## 🔧 개발

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

---

## 🤝 기여

이슈와 PR을 환영합니다!

1. Fork
2. Feature 브랜치 생성 (`git checkout -b feature/amazing-feature`)
3. Commit (`git commit -m 'Add amazing feature'`)
4. Push (`git push origin feature/amazing-feature`)
5. Pull Request

---

## 📄 라이선스

MIT License - [LICENSE](LICENSE) 참고

---

## 🔗 링크

- [Datadog API 문서](https://docs.datadoghq.com/api/)
- [GitHub Repository](https://github.com/junyeong-ai/datadog-cli)
- [Issue Tracker](https://github.com/junyeong-ai/datadog-cli/issues)

---

**For AI Agents**: See [CLAUDE.md](CLAUDE.md)
