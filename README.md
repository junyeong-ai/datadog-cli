# Datadog CLI

[English](README.en.md) · [명령 참조](.claude/skills/datadog-query/reference.md)

Datadog의 로그, 메트릭, APM, RUM과 운영 리소스를 조회하는 Rust CLI입니다.
`1h`·`now-1h` 같은 상대 시간과 RFC3339 입력, JSON·JSONL·표 출력을 지원합니다. 조회 명령은 Datadog 리소스를 변경하지 않습니다.

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
DATADOG_CLI_VERSION=0.9.5 bash install.sh
```

소스 빌드는 Rustup과 C 빌드 도구가 필요합니다. 개발 도구체인은 `rust-toolchain.toml`의 Rust 1.99.1이며, 최소 지원 버전은 Rust 1.97입니다.

```bash
git clone https://github.com/junyeong-ai/datadog-cli
cd datadog-cli
./scripts/install.sh --source
```

스크립트 설치는 `~/.local/bin`에 바이너리를 설치합니다. 제거는 `./scripts/uninstall.sh`를 사용합니다. 설정과 스킬 삭제는 별도로 선택하며, 스킬 삭제를 선택하면 백업 없이 제거합니다.
Cargo로 바이너리만 관리하려면 `cargo install --path . --locked --target host-tuple`로 설치하고 `cargo uninstall datadog-cli`로 제거하세요. Cargo의 기본 설치 경로는 `~/.cargo/bin`입니다. 두 설치 방식을 함께 사용하지 마세요.

설치 경로는 `INSTALL_DIR`, 스킬 경로는 `DATADOG_CLI_SKILL_DIR`로 지정할 수 있습니다. 제거 시에도 같은 경로를 지정하세요.

업그레이드할 때 확인할 변경 사항과 이전 방법은 [CHANGELOG](CHANGELOG.md)에 있습니다.

## 시작하기

```bash
datadog-cli config init
datadog-cli config edit
datadog-cli logs search "service:api status:error" --from 1h
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

## 조회 예제

```bash
datadog-cli logs aggregate "service:api status:error" --from 6h
datadog-cli logs timeseries "status:error" --from 24h --interval 1h
datadog-cli timeseries "sum:errors{*}" "sum:hits{*}" --formula "a / b * 100"
datadog-cli scalar "avg:system.cpu.user{*} by {host}" --aggregator avg
datadog-cli spans "service:api error:true" --from 30m
datadog-cli rum "@type:error" --from 1h
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
datadog-cli llm-obs --ml-app chatbot --span-kind llm
```

시간은 `now`, 과거 기준 상대 시간(`15m`, `1h`, `7d`, `now-1h`, `2 hours ago`, 단위 s·m·h·d·w), RFC3339 또는 Unix **초**로 입력합니다. 상대 시간은 항상 기준 시각 이전을 뜻하며, 그 밖의 형식은 CLI 문법 오류(종료 코드 2)입니다. 시각은 1970-01-01T00:00:00Z부터 9999-12-31T23:59:59Z까지입니다. 시작 시간이 종료 시간보다 늦거나 상대 시간이 이 범위를 벗어나면 자격증명을 사용하기 전에 종료 코드 1로 실패합니다.
`metrics`는 쿼리를 수정 없이 전달합니다. 집계 방식과 간격은 `metrics "avg:system.cpu.user{*}.rollup(avg, 3600)"`처럼 쿼리에 지정하고, 여러 쿼리의 간격은 `timeseries --interval 3600`처럼 초 단위로 지정합니다. 간격은 반환 포인트 수의 상한을 보장하지 않습니다.
LLM Observability는 Agent Observability 프리뷰 API에 의존합니다. 위치 인자로 쿼리를 전달하거나 `--ml-app`·`--span-kind`로 필터링할 수 있으며, 두 방식은 함께 사용할 수 없습니다. 쿼리는 수정 없이 전송합니다.
Rust의 `search_llm_obs_spans`는 `LlmObsSearchParams`와 `LlmObsFilter`로 조회 방식을 구분합니다. 인시던트 등 제품별 기능은 계정의 제품 활성화와 권한이 필요합니다.

## API 탐색과 실행

API 기능과 요청 스키마는 인증 없이 탐색할 수 있습니다. 목록은 고정된 공식 명세를 기준으로 하며, 현재 조직의 제품 활성화나 사용자 권한을 검사하지 않습니다.
기능 목록은 기본 25개씩 반환합니다. `--search`는 ID·설명에 대한 대소문자 구분 없는 문자열 검색이며, `--offset`·`--limit`으로 다음 목록을 읽습니다.

```bash
datadog-cli capabilities domains
datadog-cli capabilities list --domain ci-visibility-pipelines
datadog-cli capabilities show v2.SearchCIAppPipelineEvents
```

검토된 API는 `query DOMAIN VERSION.OPERATION --input JSON`으로 실행합니다. 입력의 `path`·`query`·`headers`·`body`는 해당 공식 API 구조를 따르며, 인증 실행 전에 스키마를 검증합니다. 속성을 선언한 객체는 선언되지 않은 필드를 거부하므로, 필드 이름의 오타가 필터 없는 조회로 이어지지 않습니다. `--input @request.json` 또는 `--input -`로 파일·표준 입력을 사용할 수 있습니다.

```bash
datadog-cli query containers v2.ListContainers
datadog-cli query metrics v1.GetMetricMetadata --input '{"path":{"metric_name":"system.cpu.user"}}'
```

`query`의 `.data`는 변형하지 않은 API 응답이며 전용 조회 명령의 출력 경로와 다릅니다. `.meta`에는 명세 출처·HTTP 상태·API 메타데이터·실행 비용을 보존합니다. 응답이 번들된 명세와 다르면 데이터는 그대로 반환하고, 위반 위치와 기대 조건을 `.meta.result.schema_violations`와 stderr 경고로 알립니다. 해당 위치의 필드는 문서와 다른 타입이거나 null일 수 있습니다. CLI가 해석하는 페이지 커서·처리 상태·DDSQL 상태가 잘못되면 종료 코드 7입니다. JSONL은 CLI 메타데이터 없이 API 응답을 출력하며, 최상위가 배열인 응답은 원소마다 한 줄씩 출력합니다. 단일 응답은 전체 검색 결과 수집 완료를 뜻하지 않습니다.

기본 제한은 인증·API 실행 60초, 재시도 포함 4회 호출, 누적 응답 본문 8 MiB입니다. `--deadline`·`--max-calls`·`--max-bytes`로 조정할 수 있습니다. CSV·바이너리는 `--output PATH`가 필요하며 기존 파일을 덮어쓰지 않습니다. 제품 활성화·권한·사이트 지원은 API에 따라 다릅니다. [명세와 지원 정책](api/README.md)을 참고하세요. 자격증명이나 전체 인증 설정을 반환하는 API는 제외합니다. Webhook URL에는 Basic 인증 정보가 포함될 수 있어 전체 설정 조회도 제외합니다. Synthetic 조회는 전용 실행 결과·이력 API를 사용합니다.

`capabilities show`에 `time_range`가 있는 작업은 `--from 1h --to now`를 지원합니다. 하나의 기준 시각으로 계산한 뒤 해당 API의 초·밀리초·RFC3339 필드에 넣으며, 입력 JSON의 시간 필드와 함께 지정하면 오류입니다. 그 외 작업은 공식 입력 구조를 그대로 사용합니다. `.meta.context.time_range`는 정규화한 요청 시간 범위입니다.

`--select /data`처럼 JSON Pointer를 지정하면 해당 값만 포인터를 키로 하는 객체에 반환합니다. 누락된 경로는 오류이며, API 메타데이터는 유지합니다. `.meta.result.processing`은 명시적으로 확인한 서버 상태에 따라 `complete`·`partial`·`unknown`이며 페이지 수집 완료와 별개입니다.

여러 페이지는 커서 계약을 검증한 작업에서 `--max-pages N`으로 요청합니다. 시간 기반 검색에는 고정된 범위가 필요합니다. 원본 응답은 `.data.pages`에 보존하며, 다음 커서와 서버 처리 상태를 구분합니다. `--wait`는 DDSQL 작업을 같은 시간·호출·바이트 한도 안에서 조회하며 제출 요청을 재시도하지 않습니다.

## MCP 전문 도구

공식 MCP 전문 도구는 `mcp list --toolset profiling`으로 오프라인 탐색하고 `mcp show TOOL`로 현재 서버 스키마를 확인한 뒤 `mcp call TOOL --input JSON`으로 호출합니다. DBM·프로파일링·Kubernetes·데이터 계보 등의 조회·분석 도구를 명시적으로 검토한 목록으로 제한합니다. 같은 인증·사이트 설정을 사용하며 REST와 MCP를 자동 전환하지 않습니다. 초기화·검색·실행·정리를 모두 포함하는 호출 수·시간·응답 바이트 한도가 적용됩니다. 도구의 실제 제공 여부와 제품 권한은 조직별로 다릅니다. 서버가 제공하는 분석은 원본 도구 결과로 보존하며, 독립적으로 확인한 사실과 구분해야 합니다.

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
CLI는 OAuth 조회 명령마다 `pup auth token --org=<세션>`을 한 번 실행합니다. Pup이 저장·갱신을 담당하며 이 CLI는 토큰을 파일에 저장하지 않습니다. 같은 사이트의 CLI 자격증명 프로세스는 OS 파일 잠금으로 순서대로 실행해 동시 갱신 충돌을 방지합니다. 토큰은 저장하지 않으며, 잠금 대기를 포함해 제공자 실행을 60초로 제한하고 실패·빈 값·잘못된 응답은 인증 오류(3)로 처리합니다. 다른 키·토큰·세션으로 자동 전환하지 않습니다.
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

## 출력과 자동화

`--format`은 API·탐색 명령의 앞뒤 어디에서나 사용할 수 있습니다. config 명령은 텍스트만 출력하며 `--format`을 지정하면 종료 코드 1로 실패합니다.
JSON은 터미널에서 들여쓰기하고 파이프에서는 압축합니다. JSONL은 `data` 항목만 한 줄씩 출력하므로 페이지 메타데이터가 없습니다.
표는 객체 또는 객체 배열을 표시합니다. 진단 로그는 stderr로 출력합니다.
로그의 사용자 속성은 `.data[].attributes`에 보존하며 추적 ID의 실제 키는 계측 설정에 따라 다릅니다.
Dashboard 목록·상세는 API의 선택 설정과 확장 필드를 보존합니다. 상세 작성자는 원본 `author_handle`·`author_name`과 `author` 별칭으로 조회할 수 있고 `created`·`modified` 별칭도 제공하며, 템플릿 변수의 `defaults`·`type`과 위젯의 `is_column_break`를 유지합니다.
모니터 상세는 평가 설정·복구 임계값·확장 필드를, 호스트는 ID·별칭·서버가 제공한 메타데이터와 메트릭을 보존합니다. Span의 HTTP·메시징 속성은 유지하며 `--full-stack-trace`는 오류 스택에만 적용합니다.
메트릭의 `.data[].units`는 기본 단위와 분모 단위의 순서·null·배율을 보존합니다. 기존 `.unit`은 첫 번째 비어 있지 않은 단위의 요약입니다. API가 보고한 메트릭 처리 오류는 HTTP 200 응답이어도 종료 코드 4, 잘못된 응답 구조는 7을 반환합니다.
`scalar`는 API 열 배열을 `.data.columns`에 순서대로 보존하며 같은 이름의 열도 덮어쓰지 않습니다. 숫자 값은 `jq '.data.columns[] | select(.type == "number") | .values[]'`로 추출하고, JSONL에서는 `.data` 접두어를 제외합니다. 표 헤더는 `[순번] 이름 (유형)`으로 열을 구분합니다.
RUM 상세 속성은 `.data[].attributes`에, Incident의 `--include` 리소스는 `.meta.included`에 있습니다.
조회 API의 상태·경고·요청 ID는 `.meta.api`에 보존합니다. 서버 timeout과 응답 경고는 모든 출력 형식에서 stderr에도 표시됩니다. 비치명 경고가 있는 부분 결과도 종료 코드 0을 반환하므로, 경고가 있는 집계를 전체 건수로 해석하지 마세요.

```bash
datadog-cli logs search "status:error" --format jsonl | jq -r '.service'
datadog-cli logs aggregate "status:error" --group-by service --group-limit 10
datadog-cli monitors list --format table
datadog-cli spans "service:api" --limit 100 --format json > page.json
jq -r '.pagination.next_cursor // empty' page.json
```

`logs aggregate`와 `logs timeseries`의 `--group-by`는 서버에서 로그 수 내림차순으로 최대 `--group-limit`개 그룹을 구합니다. 전체 그룹 전수 조회가 아니며 서버 경고는 결과와 함께 확인해야 합니다.

목록 명령은 한 페이지를 반환하고, 모든 명령이 같은 `.pagination` 구조를 씁니다. 페이지 크기는 `--limit`입니다. 다음 페이지 값은 API 방식에 따라 `next_cursor`·`next_offset`·`next_page` 중 하나이며, 같은 이름의 `--cursor`·`--offset`·`--page`로 전달합니다. 다음 페이지가 없으면 그 값은 `null`입니다.
`pagination.returned`는 이번 응답 건수이며 `total_count`는 서버가 알려준 전체 건수입니다. 알 수 없으면 `null`입니다.
`has_next_is_estimate`가 `true`이면 다음 페이지 여부는 페이지 크기로 추정한 값입니다.
반복 조회에서는 절대 시작·종료 시간을 고정해 같은 기간을 조회하세요.
`--tag-filter "env:,service:"`는 해당 접두어만 유지하고 `""`는 모든 태그를 제외하며 `"*"`는 모두 유지합니다.

`--error-format json`은 최종 런타임 오류를 stderr의 JSON 한 줄로 출력합니다. `.error.kind`는 로컬 자격증명 부재·OAuth 제공자 실패(`credentials`), Datadog의 자격증명 거부(`authentication`, HTTP 401), 권한 부족(`permission_denied`, HTTP 403)을 구분하며 모두 종료 코드 3입니다. 재시도 로그 등 다른 진단이 앞에 올 수 있으며 CLI 문법 오류는 텍스트입니다. stderr가 닫혀 경고나 최종 오류를 쓰지 못해도 결과와 원래 종료 코드는 유지됩니다.

종료 코드: `0` 성공·닫힌 stdout 파이프, `1` 입력·설정·기타 오류, `2` CLI 문법,
`3` 자격증명·인증·권한, `4` API, `5` 요청 제한, `6` 네트워크·타임아웃, `7` 응답 디코딩, `130` Ctrl+C.

## 에이전트와 개발

조회용 [스킬](.claude/skills/datadog-query/SKILL.md)은 설치 스크립트로 `~/.claude/skills/datadog-query`에 설치하거나, 사용하는 에이전트의 스킬 디렉터리에 해당 폴더를 복사할 수 있습니다. Claude Code에서 스킬은 호출된 턴 동안 `datadog-cli`와 `jq` 명령만 사전 승인합니다.
개발 규칙은 [AGENTS.md](AGENTS.md), 조사 예제는 [examples.md](.claude/skills/datadog-query/examples.md)를 참고하세요.

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo deny check
cargo run --locked --example command_reference > .claude/skills/datadog-query/reference.md
```

CI는 Linux·macOS·Windows 테스트, MSRV 검사, 워크플로우 보안 감사와 의존성 검사를 수행합니다.
