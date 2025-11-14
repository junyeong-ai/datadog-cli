# Datadog CLI

[![Rust](https://img.shields.io/badge/rust-1.91.1%2B%20(2024%20edition)-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![Tests](https://img.shields.io/badge/tests-122%20passing-green?style=flat-square)](https://github.com/junyeong-ai/datadog-cli)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

> Datadog을 명령줄에서 빠르게 조회하는 고성능 CLI 도구

[한국어](README.md) | [English](README.en.md)

---

## ✨ 주요 기능

- 🚀 **5.1MB 단일 바이너리** - 의존성 없음, 즉시 실행
- 📊 **10개 명령어** - metrics, logs, monitors, events, hosts, dashboards, spans, services, rum, config
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
- 바이너리가 `~/.local/bin/datadog`에 설치됩니다
- **Claude Code AI 스킬** 설치 여부를 선택할 수 있습니다 (선택 1 권장)

### 2. 설정
```bash
datadog config init
datadog config edit
```

### 3. 사용
```bash
datadog monitors list
datadog metrics "avg:system.cpu.user{*}" --from "1 hour ago"
datadog logs search "status:error" --limit 10
```

완료! 🎉

---

## 🤖 Claude Code AI 스킬

이 프로젝트는 [Claude Code](https://code.claude.com)용 AI 스킬을 포함합니다. 스킬을 설치하면 Claude가 자동으로 Datadog 쿼리를 실행해줍니다!

### 주요 기능
- 🔍 **자동 쿼리 실행** - 자연어로 요청하면 Claude가 적절한 명령어를 실행
- 📊 **데이터 분석** - 에러 조사, 성능 분석, 모니터 확인 등 자동화
- 🎯 **컨텍스트 인식** - 프로젝트 컨텍스트에 맞는 쿼리 자동 생성
- 🛠️ **Unix 파이프라인** - jq/grep과 조합하여 복잡한 데이터 처리

### 스킬 설치 옵션

`./install.sh` 실행 시 4가지 옵션을 선택할 수 있습니다:

**[1] Skip** - 스킬 설치 안 함
- CLI는 정상 작동
- Claude 자동 실행 기능은 사용 불가
- 나중에 다시 설치 가능

**[2] User (권장)** - 사용자 레벨 설치
- 설치 위치: `~/.claude/skills/datadog-query/`
- 모든 프로젝트에서 사용 가능
- 프로젝트 삭제해도 스킬 유지

**[3] Project** - 프로젝트 레벨만 사용
- 설치 위치: `.claude/skills/datadog-query/` (이미 있음)
- 이 프로젝트에서만 사용 가능
- 추가 설치 없음

**[4] Both** - 양쪽 모두 설치
- 사용자 레벨 + 프로젝트 레벨
- 최대 호환성

### 사용 예시

```
You: "최근 1시간 동안 production 환경의 에러 로그 보여줘"

Claude: datadog logs search "status:error env:production" --from "1 hour ago" 실행
        → 결과 분석 및 요약 제공
```

```
You: "API 서버 CPU 사용량 추이 확인해줘"

Claude: datadog metrics "avg:system.cpu.user{service:api}" --from "24 hours ago" 실행
        → 그래프 데이터 분석 및 인사이트 제공
```

### 버전 관리

- 스킬 버전: v0.1.0 (CLI 버전과 동기화)
- 설치 스크립트가 자동으로 버전 확인
- 업데이트 시 기존 버전 자동 백업 (예: `~/.claude/skills/datadog-query.bak-20251114-102030`)

---

## 💡 왜 Datadog CLI인가?

| 기능 | Web UI | Python SDK | curl | Datadog CLI |
|------|--------|-----------|------|-------------|
| 조회 속도 | 브라우저 로딩 | 10분+ 셋업 | 매번 헤더 | ✅ 즉시 (1초 이내) |
| 자동화 | ❌ 불가능 | 가능 | 가능 | ✅ 스크립트 가능 |
| 설치 | - | pip + 의존성 | 내장 | ✅ 단일 바이너리 |
| 데이터 처리 | 수동 복사 | Python 코드 | 원시 JSON | ✅ Unix 도구 연계 |

---

## 📋 명령어

### Metrics & Infrastructure
```bash
datadog metrics <query>              # 메트릭 조회
datadog hosts [options]              # 호스트 리스트
```

### Logs & Analytics
```bash
datadog logs search <query>          # 로그 검색 (기본)
datadog logs aggregate [options]     # 로그 집계 (count만 지원)
datadog logs timeseries [options]    # 로그 시계열 분석
```

### Monitoring & Events
```bash
datadog monitors list                # 모니터 리스트 (서브커맨드)
datadog monitors get <id>            # 모니터 상세 정보 (서브커맨드)
datadog events [options]             # 이벤트 조회
```

### Dashboards
```bash
datadog dashboards list              # 대시보드 리스트 (서브커맨드)
datadog dashboards get <id>          # 대시보드 상세 정보 (서브커맨드)
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
datadog config <subcommand>          # 설정 관리 (init/show/path/edit)
```

**참고**: logs, monitors, dashboards, config 명령어는 서브커맨드를 사용합니다.

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
# 파라미터로 전달
datadog logs search "status:error" --tag-filter "env:,service:"

# 전략
--tag-filter "*"                    # 모든 태그 (기본값)
--tag-filter ""                     # 태그 제외
--tag-filter "env:,service:"        # 특정 prefix만 (권장!)
--tag-filter "env:production"       # 특정 값만
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

### TOML 설정 파일

**위치:** `~/.config/datadog-cli/config.toml`

```toml
api_key = "your-api-key"
app_key = "your-app-key"
site = "datadoghq.com"  # or datadoghq.eu, us3.datadoghq.com, etc.
```

**API 키 획득**: [Datadog API Keys](https://app.datadoghq.com/organization-settings/api-keys)에서 API Key와 Application Key를 생성하세요.

**권한:** Unix 시스템에서는 600 (owner read/write only)으로 자동 설정됩니다.

### 설정 관리 명령어
```bash
# 설정 파일 생성
datadog config init

# 현재 설정 확인 (API 키 마스킹)
datadog config show

# 설정 파일 경로
datadog config path

# 설정 파일 편집 ($EDITOR 사용)
datadog config edit
```

### Datadog 사이트 설정

`site` 필드 값: `datadoghq.com` (US1, 기본), `datadoghq.eu` (EU), `us3.datadoghq.com`, `us5.datadoghq.com`, `ddog-gov.com` (US1-FED)

---

## 📦 설치 & 제거

### 설치
```bash
./install.sh
```

**설치 항목:**
1. **CLI 바이너리**: `~/.local/bin/datadog`
2. **Claude Code 스킬** (선택 사항):
   - 옵션 1: 사용자 레벨 (`~/.claude/skills/datadog-query/`) - 권장
   - 옵션 2: 프로젝트 레벨만 (`.claude/skills/datadog-query/`)
   - 옵션 3: 설치 안 함

설치 스크립트가 대화형으로 선택지를 제공합니다.

### 제거
```bash
./uninstall.sh
```

**제거 범위:**
- ✅ 바이너리 (`~/.local/bin/datadog`)
- ✅ 전역 설정 (`~/.config/datadog-cli/`) - 선택적
- ⚠️ Claude Code 스킬은 수동으로 제거:
  ```bash
  rm -rf ~/.claude/skills/datadog-query
  ```

---

## 🛠️ 개발

### 빌드
```bash
# 개발 빌드
cargo build

# 릴리즈 빌드 (최적화)
cargo build --release
# 결과: target/release/datadog (5.1MB)
```

### 테스트
```bash
cargo test              # 122 tests
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
| **테스트** | 122개 (100% 통과) |
| **의존성** | 13개 (production) |
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
