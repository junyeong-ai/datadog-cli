# Datadog CLI

[English](README.en.md) · [명령 참조](.claude/skills/datadog-query/reference.md)

Datadog의 로그, 메트릭, APM, RUM과 운영 리소스를 조회하는 Rust CLI입니다.
자연어 시간 입력과 JSON·JSONL·표 출력을 지원합니다. 조회 명령은 Datadog 리소스를 변경하지 않습니다.

## 설치

Linux x86_64·ARM64, macOS Apple Silicon·Intel, Windows x86_64 바이너리를 [Releases](https://github.com/junyeong-ai/datadog-cli/releases)에서 제공합니다.
Windows에서는 ZIP을 직접 설치합니다. Linux·macOS에서는 다음 설치 스크립트도 사용할 수 있습니다.

```bash
curl -fsSL https://raw.githubusercontent.com/junyeong-ai/datadog-cli/main/scripts/install.sh | bash
```

특정 릴리스는 `DATADOG_CLI_VERSION`으로 선택합니다. 스크립트는 체크섬 검증에 실패하면 설치를 중단합니다.
바이너리와 함께 설치하는 스킬은 같은 릴리스 태그에서 가져옵니다. 비대화형 실행에서는 바이너리와 사용자 스킬을 기본 설치합니다.

```bash
curl -fsSL https://raw.githubusercontent.com/junyeong-ai/datadog-cli/main/scripts/install.sh -o install.sh
DATADOG_CLI_VERSION=0.3.0 bash install.sh
```

소스 빌드는 Rustup과 C 빌드 도구가 필요합니다. 개발 도구체인은 `rust-toolchain.toml`의 Rust 1.99.1이며, 최소 지원 버전은 Rust 1.97입니다.

```bash
git clone https://github.com/junyeong-ai/datadog-cli
cd datadog-cli
cargo install --path . --locked
# 설치 스크립트로 로컬 스킬도 함께 설치하려면:
# ./scripts/install.sh --source
```

제거는 `./scripts/uninstall.sh`를 사용합니다. 설정과 스킬 삭제는 별도로 선택하며, 스킬 삭제 전 백업을 만듭니다.

## 시작하기

```bash
datadog-cli config init
datadog-cli config edit
datadog-cli logs search "service:api status:error" --from "1 hour ago"
datadog-cli metrics "avg:system.cpu.user{*}"
datadog-cli monitors list
```

API 키와 Application 키 또는 토큰을 설정합니다. 환경 변수는 셸에서 내보내야 하며 `.env` 파일을 자동으로 읽지 않습니다.

```bash
export DD_API_KEY="your-api-key"
export DD_APP_KEY="your-app-key"
export DD_SITE="datadoghq.com"
# 키 쌍 대신 토큰 사용:
# export DD_TOKEN="your-token"
```

설치 경로는 `INSTALL_DIR`, 스킬 경로는 `DATADOG_CLI_SKILL_DIR`로 지정할 수 있습니다. 제거 시에도 같은 경로를 지정하세요.

## 설정

설정은 **전역 파일 → 가장 가까운 프로젝트 파일 → 환경 변수 → CLI 인자** 순서로 필드별 병합합니다.
프로젝트 파일은 현재 디렉터리에서 상위 디렉터리로 탐색하는 `.datadog.toml`입니다.
전역 파일은 `~/.config/datadog-cli/config.toml`입니다.

같은 출처에 토큰과 키가 모두 있으면 비어 있지 않은 토큰을 사용합니다. 상위 출처에 키를 명시하면 하위 출처의 토큰을 대체합니다.
키의 누락된 필드는 하위 설정에서 상속하며, 빈 토큰은 하위 토큰을 해제합니다. 빈 키는 해당 키를 지웁니다.
잘못된 TOML, 알 수 없는 설정 필드, 유효하지 않은 출력 형식과 0초 타임아웃은 오류입니다.

```toml
api_key = "your-api-key"
app_key = "your-app-key"
# token = "your-token"
site = "datadoghq.com"

[defaults]
format = "json"
# tag_filter = "env:,service:"

[network]
timeout_secs = 30
total_timeout_secs = 120
max_retries = 3
```

`timeout_secs`는 한 번의 요청, `total_timeout_secs`는 재시도와 대기를 포함한 API 호출 전체의 제한입니다.
일시적 네트워크 실패·408·429·5xx만 재시도합니다. 지연은 최대 30초의 지수 백오프에 jitter를 적용합니다.
429의 `x-ratelimit-reset` 또는 `Retry-After`는 서버 대기 시간을 따르며, 30초를 넘으면 즉시 오류를 반환합니다.
리디렉션은 따르지 않습니다. HTTP(S) 환경 변수 및 OS 프록시 설정을 지원합니다.

```bash
datadog-cli config show   # 비밀값을 마스킹한 유효 설정
datadog-cli config path   # 프로젝트 파일이 있으면 해당 경로, 아니면 전역 경로
datadog-cli config edit   # 위 경로를 $EDITOR로 열기; 기본값 vim
```

`config init`은 전역 파일을 만듭니다. `DD_TAG_FILTER`는 기본 태그 필터, `RUST_LOG`는 로그 수준을 설정합니다.
지원 사이트와 전체 옵션은 [명령 참조](.claude/skills/datadog-query/reference.md)를 확인하세요.

## 0.3 업그레이드

기존 설정의 `[defaults]`에 `time_range`, `limit`, `page_size`가 남아 있으면 삭제하세요. 이 필드는 이전 버전에서도 조회에 적용되지 않았으며, 이제 알 수 없는 필드로 오류가 발생합니다. 시간과 개수는 명령 옵션으로 지정합니다.
상세·집계 JSONL은 `data` 본문만 출력합니다. 메타데이터가 필요한 스크립트는 `--format json`을 사용하세요.

## 조회 예제

```bash
datadog-cli logs aggregate "service:api status:error" --from "6 hours ago"
datadog-cli logs timeseries "status:error" --from "24 hours ago" --interval 1h
datadog-cli timeseries "sum:errors{*}" "sum:hits{*}" --formula "a / b * 100"
datadog-cli scalar "avg:system.cpu.user{*} by {host}" --aggregator avg
datadog-cli spans "service:api error:true" --from "30 minutes ago"
datadog-cli rum "@type:error" --from "1 hour ago"
datadog-cli services --kind service --owner platform-team
datadog-cli events "source:alert status:error"
datadog-cli slo list --query checkout
datadog-cli incidents list
datadog-cli error-tracking search "service:api" --track trace
datadog-cli downtimes --current-only
datadog-cli hosts --filter "env:production"
datadog-cli dashboards list
datadog-cli teams --keyword platform
datadog-cli audit "@action:login"
datadog-cli llm-obs "@ml_app:chatbot" --span-kind llm
```

시간은 `now`, `1 hour ago`, `30m ago`, RFC3339 또는 Unix **초**로 입력합니다. 시작 시간은 종료 시간보다 늦을 수 없습니다.
LLM Observability는 프리뷰 API에 의존합니다. 인시던트 등 제품별 기능은 계정의 제품 활성화와 권한이 필요합니다.

## 출력과 자동화

`--format`은 하위 명령 앞뒤 어디에서나 사용할 수 있습니다.
JSON은 터미널에서 들여쓰기하고 파이프에서는 압축합니다. JSONL은 `data` 항목만 한 줄씩 출력하므로 페이지 메타데이터가 없습니다.
표는 객체 또는 객체 배열을 표시합니다. 진단 로그는 stderr로 출력합니다.

```bash
datadog-cli logs search "status:error" --format jsonl | jq -r '.service'
datadog-cli monitors list --format table
datadog-cli spans "service:api" --limit 100 --format json > page.json
jq -r '.pagination.next_cursor // empty' page.json
```

검색 결과는 한 페이지입니다. 다음 페이지는 반환된 커서를 `--cursor`로 전달합니다.
반복 조회에서는 절대 시작·종료 시간을 고정해 같은 기간을 조회하세요. API별 페이지 방식은 명령 참조를 확인하세요.
`--tag-filter "env:,service:"`는 해당 접두어만 유지하고 `""`는 모든 태그를 제외하며 `"*"`는 모두 유지합니다.

종료 코드: `0` 성공·닫힌 stdout 파이프, `1` 입력·설정·기타 오류, `2` CLI 문법,
`3` 인증, `4` API, `5` 요청 제한, `6` 네트워크·타임아웃, `7` 응답 디코딩, `130` Ctrl+C.

## 에이전트와 개발

조회용 [스킬](.claude/skills/datadog-query/SKILL.md)은 설치 스크립트로 `~/.claude/skills/datadog-query`에 설치하거나, 사용하는 에이전트의 스킬 디렉터리에 해당 폴더를 복사할 수 있습니다.
개발 규칙은 [AGENTS.md](AGENTS.md), 조사 예제는 [examples.md](.claude/skills/datadog-query/examples.md)를 참고하세요.

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo deny check
cargo run --locked --example command_reference > .claude/skills/datadog-query/reference.md
```

벤치마크는 `cargo bench --bench parsing`으로 실행합니다. CI는 Linux·macOS·Windows 테스트, MSRV 검사, 커버리지와 의존성 검사를 수행합니다.
