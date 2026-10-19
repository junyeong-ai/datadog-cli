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
바이너리와 함께 설치하는 스킬은 같은 릴리스 태그에서 가져옵니다. 비대화형 실행에서는 바이너리와 사용자 스킬을 기본 설치합니다. 기존 스킬은 백업 없이 교체합니다.

```bash
curl -fsSL https://raw.githubusercontent.com/junyeong-ai/datadog-cli/main/scripts/install.sh -o install.sh
DATADOG_CLI_VERSION=0.8.0 bash install.sh
```

소스 빌드는 Rustup과 C 빌드 도구가 필요합니다. 개발 도구체인은 `rust-toolchain.toml`의 Rust 1.99.1이며, 최소 지원 버전은 Rust 1.97입니다.

```bash
git clone https://github.com/junyeong-ai/datadog-cli
cd datadog-cli
./scripts/install.sh --source
```

스크립트 설치는 `~/.local/bin`에 바이너리를 설치합니다. 제거는 `./scripts/uninstall.sh`를 사용합니다. 설정과 스킬 삭제는 별도로 선택하며, 스킬 삭제를 선택하면 백업 없이 제거합니다.
Cargo로 바이너리만 관리하려면 `cargo install --path . --locked --target host-tuple`로 설치하고 `cargo uninstall datadog-cli`로 제거하세요. Cargo의 기본 설치 경로는 `~/.cargo/bin`입니다. 두 설치 방식을 함께 사용하지 마세요.

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

## OAuth 인증

[Pup 2.0+](https://github.com/DataDog/pup)의 OAuth 로그인과 갱신을 사용할 수 있습니다. Pup을 PATH에 설치한 뒤 필요한 읽기 권한으로 이름 있는 세션을 만드세요. 다음은 로그·메트릭 조회 예시입니다.

```bash
pup auth login --site datadoghq.com --org cli-read --scopes timeseries_query,logs_read_data
```

전역 `~/.config/datadog-cli/config.toml`에 기본 인증 방식을 지정합니다. `site`는 최상위 필드이며 로그인한 사이트와 같아야 합니다.

```toml
site = "datadoghq.com"

[auth]
method = "oauth"
pup_org = "cli-read"
```

이후에는 인증 옵션 없이 조회합니다.

```bash
datadog-cli logs search "service:api"
datadog-cli config show
```

`method`는 `api-keys`·`token`·`oauth` 중 하나입니다. `api-keys`는 기존 `api_key`·`app_key`, `token`은 `token` 또는 `DD_TOKEN`을 사용합니다. `oauth`는 Pup 세션을 사용하며 정적 키·토큰은 사용하지 않습니다. 환경 변수는 선택한 방식의 자격증명 값만 덮어씁니다. `[auth]`가 없으면 기존 토큰·키 우선순위를 유지합니다.
`pup_org`는 Pup에 저장한 세션 이름이며 실제 Datadog 조직은 브라우저 승인 과정에서 선택합니다. 다른 명령에는 해당 API의 scope와 사용자 권한이 필요합니다.
한 명령만 다른 방식으로 실행하려면 `--auth-method api-keys|token|oauth`를 지정합니다. CLI에 직접 지정한 키·토큰도 전역 기본 방식보다 우선합니다. `--auth-method`와 맞지 않는 자격증명 옵션은 오류입니다. OAuth 세션은 `--pup-org`, 사이트는 `--site` 또는 `DD_SITE`로 재정의합니다. 기존 `--auth-provider pup`는 `--auth-method oauth`와 같은 방식으로 계속 사용할 수 있습니다.
`[auth]`는 전역 파일에서만 허용합니다. 프로젝트 `.datadog.toml`이 다른 OAuth 사이트를 지정하면 오류이며, 변경하려면 `--site` 또는 `DD_SITE`로 명시해야 합니다. 임의 자격증명 명령은 설정할 수 없습니다.
CLI는 OAuth 조회 명령마다 `pup auth token --org=<세션>`을 한 번 실행합니다. Pup이 저장·갱신을 담당하며 이 CLI는 토큰을 파일에 저장하지 않습니다. 제공자 실행은 60초로 제한하고 실패·빈 값·잘못된 응답은 인증 오류(3)로 처리합니다. 다른 키·토큰·세션으로 자동 전환하지 않습니다.
`config show`는 인증 방식·선택 출처·세션과 마스킹한 설정을 표시하며 Pup을 실행하지 않습니다. 인증 옵션은 API 명령과 `config show`에 적용됩니다. 무인 실행에는 SAT나 서비스 계정 키를 사용하세요.

## 설정

설정은 **전역 파일 → 가장 가까운 프로젝트 파일 → 환경 변수 → CLI 인자** 순서로 필드별 병합합니다.
프로젝트 파일은 현재 디렉터리에서 상위 디렉터리로 탐색하는 `.datadog.toml`입니다.
전역 파일은 `~/.config/datadog-cli/config.toml`입니다.
홈 경로는 Linux·macOS의 `HOME`, Windows의 `USERPROFILE`을 따릅니다.

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
재시도 가능한 응답의 `Retry-After`와 429의 `x-ratelimit-reset`은 서버 대기 시간을 지정합니다. 둘 다 있으면 더 긴 대기를 따르며, 30초를 넘으면 원래 오류를 즉시 반환합니다.
리디렉션은 따르지 않습니다. HTTP(S) 환경 변수 및 OS 프록시 설정을 지원합니다.

```bash
datadog-cli config show   # 비밀값을 마스킹한 유효 설정
datadog-cli config path   # 프로젝트 파일이 있으면 해당 경로, 아니면 전역 경로
datadog-cli config edit   # 위 경로를 $EDITOR로 열기; 기본값 vim
```

`EDITOR`는 `code --wait`처럼 인자를 포함할 수 있습니다. 공백이 있는 실행 경로는 큰따옴표로 감싸세요. Linux·macOS는 POSIX 인용 규칙, Windows는 Windows 명령줄 인용 규칙을 사용하며, 변수 확장·파이프·명령 치환은 수행하지 않습니다. 편집 중에는 편집기가 Ctrl+C를 처리하며 부모는 편집기 종료를 기다립니다. 편집기가 중단 종료하면 CLI도 130을 반환합니다.

`config init`은 전역 파일을 만듭니다. `DD_TAG_FILTER`는 기본 태그 필터, `RUST_LOG`는 로그 수준을 설정합니다.
지원 사이트와 전체 옵션은 [명령 참조](.claude/skills/datadog-query/reference.md)를 확인하세요.

## 0.8 업그레이드

전역 `[auth]`에서 기본 인증 방식을 선택할 수 있습니다. `[auth]`가 없는 기존 설정은 호환됩니다. OAuth의 필수 세션은 설정과 CLI를 합친 뒤 검사합니다. Rust에서 인증 설정을 자동 적용하려면 비동기 `Config::load_with_options(ConfigOptions)`를 사용하세요. 동기 `Config::load`는 OAuth 설정에서 비동기 로더 사용을 안내하는 오류를 반환합니다.
스킬은 백업 없이 전체 교체하며 임시 사본은 스킬 검색 경로 밖에서 준비합니다. 삭제를 선택한 스킬도 백업을 남기지 않습니다.

## 0.7 업그레이드

Dashboard 목록·상세는 API의 선택 설정과 확장 필드를 보존합니다. 상세 작성자는 원본 `author_handle`·`author_name`과 기존 `author` 별칭으로 조회할 수 있고, `created`·`modified` 별칭도 유지합니다. 템플릿 변수의 `defaults`·`type`, 위젯의 `is_column_break`와 알 수 없는 설정을 누락하지 않습니다.
Rust Dashboard 모델은 필수 core 외의 필드를 `extra` 맵에 보존합니다. `DashboardSummary`는 JSON Map 별칭이며, `AuthorInfo`·`TemplateVariable`·`WidgetLayout`은 제거됐습니다.
설정 경고와 최종 오류 메시지도 닫힌 stderr 파이프에서 중단되지 않습니다. 최종 오류 메시지를 쓰지 못해도 원래 종료 코드를 유지합니다.

## 0.6 업그레이드

RUM 상세 속성은 레코드의 `attributes`에 보존됩니다. 오류 스택은 `.data[].attributes.error.stack`, 페이지 정보는 `.data[].attributes.view`에서 확인하세요. `--full-stack-trace`는 이 오류 스택에 적용됩니다.
조회 API의 상태·경고·요청 ID 등 서버 메타데이터는 `.meta.api`에 보존합니다. 서버 timeout과 응답 경고는 모든 출력 형식에서 stderr에도 표시되며, 비치명 경고가 있는 부분 결과의 종료 코드는 0입니다. 경고가 있는 집계를 전체 건수로 해석하지 마세요.
Incident 목록·상세 조회의 `--include` 리소스는 `.meta.included`에 있습니다. 메타데이터가 필요한 스크립트는 `--format json`을 사용하세요.
Rust `RumAttributes`는 공식 API 구조를 따르며 상세 속성은 `attributes` 맵에 있습니다. 기존 `RumApplication` 등 별도 상세 모델은 제거됐고, `CursorMeta`·`CursorPage`는 확장 필드를 `extra` 맵에 보존합니다.

## 0.5 업그레이드

`scalar`는 `data.columns`에 API 열 배열을 순서대로 보존합니다. 각 열의 `type`, 선택적 `name`, `values`, 단위 메타데이터가 유지되며 같은 이름도 덮어쓰지 않습니다.
JSONL은 `columns`를 포함한 응답 본문 하나를 한 줄에 출력합니다. 표 헤더는 `[순번] 이름 (유형)`으로 열을 구분합니다.
숫자 값 추출은 `jq '.data.columns[] | select(.type == "number") | .values[]'`를 사용하세요. JSONL에서는 `.data` 접두어를 제외합니다.
API가 보고한 메트릭 처리 오류는 종료 코드 4, 잘못된 응답 구조는 7을 반환합니다.
Rust 클라이언트의 `query_scalar`·`query_timeseries`는 검증된 `ScalarAttributes`·`TimeseriesAttributes`를 직접 반환합니다.

## 0.4 업그레이드

`metrics --max-points`와 자동 rollup을 제거했습니다. `metrics`는 쿼리를 수정 없이 전달합니다.
집계 방식과 간격을 직접 지정하려면 `metrics "avg:system.cpu.user{*}.rollup(avg, 3600)"`을 사용하세요.
다중 쿼리의 간격은 `timeseries "avg:system.cpu.user{*}" "avg:system.cpu.system{*}" --interval 3600`으로 지정합니다. 간격은 초 단위이며, 반환 포인트 수의 상한을 보장하지 않습니다.

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
로그의 사용자 속성은 `.data[].attributes`에 보존하며 추적 ID의 실제 키는 계측 설정에 따라 다릅니다.
모니터 상세는 평가 설정·복구 임계값·확장 필드를 보존합니다. Span의 HTTP·메시징 속성은 유지하며 `--full-stack-trace`는 오류 스택에만 적용합니다.
메트릭의 `.data[].units`는 기본 단위와 분모 단위의 순서·null·배율을 보존합니다. 기존 `.unit`은 첫 번째 비어 있지 않은 단위의 요약입니다.

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
