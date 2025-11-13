# Datadog CLI

[![Rust](https://img.shields.io/badge/rust-1.91.1%2B%20(2024%20edition)-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![Tests](https://img.shields.io/badge/tests-117%20passing-green?style=flat-square)](https://github.com/junyeong-ai/datadog-cli)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

> Datadog을 명령줄에서 빠르게 조회하는 고성능 CLI 도구

---

## ✨ 주요 기능

- 🚀 **5.1MB 단일 바이너리** - 의존성 없음, 즉시 실행
- 📊 **13개 명령어** - metrics, logs, monitors, events, hosts, spans, services, rum, dashboards
- 🌍 **자연어 시간 지원** - "1 hour ago", "yesterday", "last week"
- 🎯 **3가지 출력 포맷** - JSON, JSONL, Table
- 🔧 **Unix 파이프라인** - grep, jq 등과 완벽 호환
- ⚡ **최적화된 성능** - HTTP/2 + rustls, 비동기 처리

---

## 🚀 빠른 시작 (3분)

### 1. 설치
```bash
./install.sh
```
바이너리가 `~/.local/bin/datadog`에 설치됩니다.

### 2. 설정
```bash
datadog config init
vim ~/.config/datadog-cli/config.toml
```

**config.toml:**
```toml
api_key = "your-api-key-here"
app_key = "your-app-key-here"
site = "datadoghq.com"
```

### 3. 사용
```bash
datadog monitors list
datadog metrics "avg:system.cpu.user{*}" --from "1 hour ago"
datadog logs search "status:error" --limit 10
```

완료! 🎉

---

## 💡 왜 Datadog CLI인가?

### vs Datadog Web UI
| 항목 | Web UI | Datadog CLI |
|------|--------|-------------|
| 조회 속도 | 브라우저 로딩 | ✅ 즉시 (1초 이내) |
| 자동화 | ❌ 불가능 | ✅ 스크립트 가능 |
| 데이터 처리 | 수동 복사 | ✅ Unix 도구 연계 |

### vs Python SDK
| 항목 | Python SDK | Datadog CLI |
|------|-----------|-------------|
| 설치 | pip, 의존성 관리 | ✅ 단일 바이너리 |
| 시작 시간 | 10분+ | ✅ 3분 |
| 메모리 | Python 런타임 | ✅ 네이티브 (낮음) |

### vs curl
| 항목 | curl | Datadog CLI |
|------|------|-------------|
| 인증 | 매번 헤더 설정 | ✅ 자동 |
| 에러 처리 | 수동 파싱 | ✅ 명확한 메시지 |
| 출력 | 원시 JSON | ✅ 포맷 선택 가능 |

---

## 📋 명령어

### Metrics & Infrastructure
```bash
datadog metrics <query>              # 메트릭 조회
datadog hosts [options]              # 호스트 리스트
```

### Logs & Analytics
```bash
datadog logs search <query>          # 로그 검색
datadog logs aggregate [options]     # 로그 집계 (count/sum/avg/min/max)
datadog logs timeseries [options]    # 로그 시계열 분석
```

### Monitoring & Events
```bash
datadog monitors list                # 모니터 리스트
datadog monitors get <id>            # 모니터 상세 정보
datadog events [options]             # 이벤트 조회
```

### Dashboards
```bash
datadog dashboards list              # 대시보드 리스트
datadog dashboards get <id>          # 대시보드 상세 정보
```

### APM & Tracing
```bash
datadog spans [options]              # APM 스팬 검색
datadog services [options]           # 서비스 카탈로그
```

### RUM (Real User Monitoring)
```bash
datadog rum [options]                # 사용자 경험 모니터링
```

### Configuration
```bash
datadog config init                  # 설정 파일 생성
datadog config show                  # 현재 설정 확인 (마스킹)
datadog config path                  # 설정 파일 경로
```

**Config file:** `~/.config/datadog-cli/config.toml`
```toml
api_key = "your-api-key"
app_key = "your-app-key"
site = "datadoghq.com"  # or datadoghq.eu, us3.datadoghq.com, etc.
```

**전체 명령어 옵션:** `datadog --help` 또는 `datadog <command> --help`

---

## 🎯 사용 예시

### 예시 1: 프로덕션 에러 모니터링
```bash
# 최근 1시간 프로덕션 에러 검색
datadog logs search "status:error env:production" \
  --from "1 hour ago" \
  --limit 50 \
  --format table
```

**결과:**
```
┌────────────────────┬─────────────────────┬───────────────────┐
│ timestamp          ┆ service             ┆ message           │
├────────────────────┼─────────────────────┼───────────────────┤
│ 2025-11-13 06:00   ┆ payment-api         ┆ Connection timeout│
│ 2025-11-13 06:02   ┆ auth-service        ┆ Invalid token     │
└────────────────────┴─────────────────────┴───────────────────┘
```

### 예시 2: CPU 사용량 추이 분석
```bash
# 지난 24시간 API 서버 CPU 사용량
datadog metrics "avg:system.cpu.user{service:api}" \
  --from "24 hours ago" \
  --to "now" \
  --format json
```

**결과:**
```json
{
  "data": [{
    "metric": "system.cpu.user",
    "points": [
      {"timestamp": "2025-11-12 06:00:00 UTC", "value": 45.2},
      {"timestamp": "2025-11-12 12:00:00 UTC", "value": 62.8}
    ]
  }]
}
```

### 예시 3: Unix 파이프라인 활용
```bash
# Alert 상태 모니터 개수 집계
datadog --format jsonl monitors list | \
  grep '"status":"Alert"' | \
  jq -s 'length'

# 출력: 42
```

**고급 예시:**
```bash
# 서비스별 에러 로그 TOP 5
datadog logs aggregate \
  --query "status:error" \
  --from "1 hour ago" \
  --compute '[{"aggregation":"count","type":"total"}]' \
  --group-by '[{"facet":"service"}]' \
  --format json | \
  jq '.data.buckets | sort_by(.count) | reverse | .[0:5]'
```

### 예시 4: 스크립트 자동화
```bash
#!/bin/bash
# 에러율 모니터링 스크립트

ERROR_COUNT=$(dd logs search "status:error" \
  --from "5 minutes ago" \
  --format json | \
  jq '.pagination.total')

if [ $ERROR_COUNT -gt 10 ]; then
  echo "⚠️  High error rate: $ERROR_COUNT errors"
  # Slack 알림 전송
  curl -X POST $SLACK_WEBHOOK -d "{\"text\":\"High error rate: $ERROR_COUNT\"}"
else
  echo "✅ Error rate normal: $ERROR_COUNT errors"
fi
```

---

## 🌟 고급 기능

### 자연어 시간 표현
```bash
# 상대 시간
datadog logs search "..." --from "10 minutes ago"
datadog logs search "..." --from "2 hours ago"
datadog logs search "..." --from "3 days ago"

# 명명된 시간
datadog logs search "..." --from "yesterday"
datadog logs search "..." --from "last week"
datadog logs search "..." --from "last month"

# 절대 시간
datadog logs search "..." --from "2025-01-15T10:30:00Z"
datadog logs search "..." --from "1704067200"  # Unix timestamp
```

### 태그 필터링
태그 필터링으로 응답 크기를 대폭 줄일 수 있습니다:

```bash
# 환경 변수로 설정
DD_TAG_FILTER="env:,service:" datadog logs search "status:error"

# 또는 파라미터로 전달
datadog logs search "status:error" --tag-filter "env:,service:"

# 전략
DD_TAG_FILTER="*"                    # 모든 태그 (기본값)
DD_TAG_FILTER=""                     # 태그 제외
DD_TAG_FILTER="env:,service:"        # 특정 prefix만 (권장!)
DD_TAG_FILTER="env:production"       # 특정 값만
```

### 출력 포맷
```bash
# JSON (기본) - API 응답 그대로
datadog monitors list --format json

# JSONL (JSON Lines) - Unix 도구 친화적
datadog monitors list --format jsonl | grep "Alert" | jq -s '.'

# Table - 사람이 읽기 쉬움
datadog monitors list --format table
```

### Unix 파이프라인 패턴
```bash
# 패턴 1: 필터링 + 집계
datadog --format jsonl monitors list | \
  grep "production" | \
  jq -s 'length'

# 패턴 2: 데이터 변환
datadog monitors list --format json | \
  jq '.data[] | {id, name, status}'

# 패턴 3: 파일 저장 후 처리
datadog monitors list > monitors.json
jq '.data | length' monitors.json
jq '.data[] | select(.status=="Alert")' monitors.json
```

---

## ⚙️ 설정

### 우선순위
설정은 다음 순서로 적용됩니다:

1. **환경 변수** (최우선)
   ```bash
   DD_API_KEY=xxx DD_APP_KEY=yyy datadog monitors list
   ```

2. **로컬 .env** (프로젝트별)
   ```bash
   # .env 파일
   DD_API_KEY=xxx
   DD_APP_KEY=yyy
   DD_SITE=datadoghq.com
   ```

3. **전역 설정** (사용자 기본값)
   ```bash
   # ~/.config/datadog-cli/config
   DD_API_KEY=xxx
   DD_APP_KEY=yyy
   DD_SITE=datadoghq.com
   ```

### 사용 가능한 환경 변수

| 변수 | 설명 | 기본값 | 필수 |
|------|------|--------|------|
| `DD_API_KEY` | Datadog API 키 | - | ✅ |
| `DD_APP_KEY` | Datadog Application 키 | - | ✅ |
| `DD_SITE` | Datadog 사이트 | `datadoghq.com` | ❌ |
| `DD_TAG_FILTER` | 태그 필터 (응답 크기 최적화) | `*` (전체) | ❌ |
| `LOG_LEVEL` | 로그 레벨 (error/warn/info/debug) | `warn` | ❌ |

**예시:**
```bash
# 전체 태그 포함 (기본)
DD_TAG_FILTER="*" datadog logs search "status:error"

# 특정 태그만 포함 (권장)
DD_TAG_FILTER="env:,service:" datadog logs search "status:error"

# 디버그 로그 활성화
LOG_LEVEL=debug datadog monitors list
```

### 설정 관리 명령어
```bash
# 현재 설정 확인 (API 키 마스킹)
datadog config show

# 설정 파일 경로
datadog config path              # 로컬 .env
datadog config path --global     # 전역 설정

# 모든 설정 소스 확인
datadog config list

# 설정 편집
datadog config edit --global     # 전역 설정 편집
```

### 설정 파일 위치

**전역 설정 (권장):**
```
~/.config/datadog-cli/config
```

**로컬 설정:**
```
.env (프로젝트 루트)
```

**템플릿:** `.env.example` 참조

### Datadog 사이트 설정

`DD_SITE` 환경 변수로 사용할 Datadog 사이트 지정:

| 사이트 | 값 | 지역 |
|-------|-----|------|
| US1 (기본) | `datadoghq.com` | 미국 |
| EU | `datadoghq.eu` | 유럽 |
| US3 | `us3.datadoghq.com` | 미국 |
| US5 | `us5.datadoghq.com` | 미국 |
| US1-FED | `ddog-gov.com` | 미국 정부 |

```bash
DD_SITE=datadoghq.eu datadog monitors list
```

### ⚠️ 중요: .env 파일
`.env`는 **프로젝트 공유 파일**입니다 (Node.js, Docker 등도 사용).

**안전한 방법:**
- ✅ **전역 설정 사용** (`~/.config/datadog-cli/config`) - datadog-cli 전용
- ⚠️ **.env 사용 시** - 프로젝트별 오버라이드만
- ❌ **.env 삭제 금지** - 다른 도구 설정 포함 가능

---

## 📦 설치 & 제거

### 설치
```bash
./install.sh
```
바이너리가 `~/.local/bin/datadog`에 설치됩니다.

### 제거
```bash
./uninstall.sh
```

**제거 범위:**
- ✅ 바이너리 (`~/.local/bin/datadog`)
- ✅ 전역 설정 (`~/.config/datadog-cli/`) - 선택적
- ❌ 로컬 .env - 수동 제거 필요

---

## 🛠️ 개발

### 빌드
```bash
# 개발 빌드
cargo build

# 릴리즈 빌드 (최적화)
cargo build --release
# 결과: target/release/dd (5.1MB)
```

### 테스트
```bash
cargo test              # 117 tests
cargo fmt --check       # 포맷 검증
cargo clippy           # 린팅
```

### 디버그
```bash
RUST_LOG=debug cargo run -- monitors list
```

---

## 📊 성능

| 메트릭 | 값 |
|--------|-----|
| **바이너리 크기** | 5.1MB |
| **테스트** | 117개 (100% 통과) |
| **의존성** | 12개 (production) |
| **빌드 최적화** | LTO + strip + opt-level 3 |
| **평균 응답 시간** | 0.6-1.2초 (API 서버 시간) |

---

## 📄 라이선스

MIT License - [LICENSE](LICENSE) 파일 참조

---

## 🤝 기여

Issues와 Pull Requests를 환영합니다!

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### 개발 가이드라인
- `cargo fmt` - 코드 포맷팅
- `cargo clippy -- -D warnings` - 린팅 (0 warnings)
- `cargo test` - 모든 테스트 통과
- AI agent 개발: [CLAUDE.md](CLAUDE.md) 참조

---

<div align="center">

**Made with 🦀 Rust**

[⭐ Star this repo](https://github.com/junyeong-ai/datadog-cli) · [🐛 Report Bug](https://github.com/junyeong-ai/datadog-cli/issues) · [✨ Request Feature](https://github.com/junyeong-ai/datadog-cli/issues)

</div>
