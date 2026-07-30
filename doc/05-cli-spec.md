# 05. CLI Spec — `oxiline` 명령줄 인터페이스

## 5.1 설계 원칙

Oxi 생태계의 규칙대로, CLI는 GUI의 부산물이 아니라 **1급 클라이언트**다. 사람이 터미널에서 직접 쓸 수도
있고, 에이전트(`oxi` 같은 터미널 AI 에이전트, 혹은 Claude/기타 LLM 툴콜)가 프로그래밍적으로 호출할 수도
있어야 한다. 이를 위해 다음 원칙을 지킨다.

1. **모든 명령은 `--json` 플래그로 기계 판독 가능한 출력을 낼 수 있다.** 사람용 출력(색상, 표, 이모지
   최소화)과 에이전트용 출력(순수 JSON, stdout 전용)을 명확히 분리한다.
2. **모든 쓰기 명령은 성공 시 변경된 리소스를 stdout에 반환한다** (에이전트가 후속 판단을 하기 위해
   ID 등을 다시 파싱할 필요가 없도록).
3. **종료 코드가 의미를 가진다** (§5.5) — 에이전트가 stdout 파싱 없이도 성공/실패를 즉시 알 수 있게.
4. **파괴적 명령(`rm`)은 기본적으로 GUI 확인 없이 즉시 실행된다** — CLI는 스크립트/에이전트용이므로
   대화형 확인 프롬프트를 기본으로 두지 않는다. 대신 `--dry-run` 플래그를 모든 쓰기 명령에 제공한다.
5. **자연어 시간 표현은 CLI 레벨에서 파싱하지 않는다** ("내일 오후 3시" 같은 파싱은 에이전트/사용자가
   호출 전에 해석해서 ISO 형식으로 넘긴다). CLI 자체에 NLP를 넣는 것은 Oxi 생태계 원칙(코어는 단순하게,
   지능은 에이전트 레이어에)에 어긋난다.

## 5.2 전체 명령어 트리

```
oxiline
├── now                                   # 지금/다음 할일 (HUD와 동일 데이터)
├── today [--date <DATE>]                 # 특정 날짜의 통합 타임라인
├── task
│   ├── add <TITLE> [옵션들]
│   ├── list [--date <DATE>|--backlog|--range <FROM>:<TO>]
│   ├── show <ID>
│   ├── done <ID>
│   ├── undone <ID>
│   ├── skip <ID>                         # 루틴 occurrence를 오늘만 건너뛰기
│   ├── edit <ID> [옵션들]
│   └── rm <ID>
├── routine
│   ├── add <TITLE> [옵션들]
│   ├── list [--active-only]
│   ├── show <ID>
│   ├── edit <ID> [옵션들]
│   ├── toggle <ID> --on|--off
│   └── rm <ID>
├── category
│   ├── add <NAME> [--hue <0-360>] [--icon <NAME>]
│   ├── list
│   └── rm <ID>
├── settings
│   ├── get [<KEY>]                       # 키 생략 시 전체 반환
│   └── set <KEY> <VALUE>
├── hud show                              # 개발/테스트용: 실행 중인 GUI에 HUD를 강제 표시
├── export --range <FROM>:<TO>            # 지정 범위 전체를 JSON으로 덤프 (읽기 전용, 항상 JSON)
└── doctor                                # DB 경로, 마이그레이션 상태, 권한 등 자가 진단
```

전역 플래그 (모든 서브커맨드 공통):

| 플래그 | 설명 |
|---|---|
| `--json` | JSON 출력 모드 (기본은 사람이 읽기 좋은 텍스트) |
| `--db <PATH>` | `OXILINE_DB_PATH` 환경 변수 대신 이 실행에 한해 DB 경로 오버라이드 |
| `--lang <ko\|en>` | 이 실행의 텍스트 출력 언어 (기본은 `settings.locale`) |
| `--dry-run` | 쓰기 명령에서 실제 반영 없이 결과 미리보기 |
| `-q`, `--quiet` | 성공 시 아무 출력도 하지 않음 (종료 코드만 확인하는 스크립트용) |

## 5.3 주요 명령어 상세

### `oxiline now`

지금 진행 중인 항목과 다음 항목을 반환한다. 플로팅 HUD가 보여주는 것과 **완전히 동일한 계산**
(`oxiline_core::timeline::get_now_context`)을 사용한다 — 즉 CLI로 "지금 뭐 해야 하지"를 물어보면
HUD와 다른 답이 나올 수 없다.

```bash
$ oxiline now --json
```
```json
{
  "current": {
    "id": "virtual:9c2e...:2026-07-30",
    "is_virtual": true,
    "title": "집중 작업 블록",
    "start_minute": 570,
    "duration_minute": 90,
    "category_id": "cat_work",
    "remaining_minute": 32
  },
  "next": {
    "id": "task_88f1...",
    "is_virtual": false,
    "title": "팀 스탠드업",
    "start_minute": 660,
    "duration_minute": 15,
    "category_id": "cat_work",
    "starts_in_minute": 60
  },
  "generated_at": "2026-07-30T10:28:00+09:00"
}
```

`current`가 없으면(자유 시간) `"current": null`이고, 사람이 읽는 모드에서는
`지금은 예정된 일이 없어요. 다음: 팀 스탠드업 (60분 후)`처럼 출력한다.

### `oxiline task add`

```bash
oxiline task add "병원 예약" --date 2026-08-02 --at 14:30 --duration 30 --category personal
oxiline task add "장보기" --backlog   # 날짜/시간 없이 백로그로
```

| 옵션 | 설명 |
|---|---|
| `--date <YYYY-MM-DD>` | 날짜 지정. `--backlog`와 배타적 |
| `--backlog` | 날짜 없는 백로그 항목으로 생성 |
| `--at <HH:MM>` | 시작 시각 (24h) |
| `--duration <MIN>` | 소요 시간(분). 기본 30 |
| `--category <ID\|NAME>` | 카테고리 (ID 또는 이름으로 매칭, 모호하면 에러) |
| `--notes <TEXT>` | 메모 |

성공 시 stdout: 생성된 `Task` JSON 전체 (`--json` 여부와 무관하게 아이디는 항상 확인 가능하도록,
사람 모드에서도 마지막 줄에 `id: task_xxxx`를 출력).

### `oxiline task list`

```bash
oxiline task list --date today --json
oxiline task list --range 2026-08-01:2026-08-07 --json
oxiline task list --backlog --json
```

`--date today` / `--date tomorrow`처럼 상대 키워드를 허용한다(`today`, `tomorrow`, `yesterday`만 —
그 이상의 자연어는 §5.1 원칙에 따라 CLI 책임이 아니다). 응답은 `TimelineItem[]` 배열
(`03-data-model.md` §3.11과 동일 스키마).

### `oxiline task done` / `skip` / `edit`

가상 occurrence의 id(`virtual:{block_id}:{date}` 형식)를 넘겨도 동작한다 — 내부적으로
`materialize_occurrence()`를 호출한 뒤 요청된 동작을 적용한다. 이는 CLI 사용자가 "지금 이 루틴은
구체화되어 있는지"를 신경 쓸 필요가 없게 하기 위함이다.

```bash
oxiline task done virtual:9c2e1a-...:2026-07-30
oxiline task skip virtual:9c2e1a-...:2026-07-30 --json
```

### `oxiline routine add`

```bash
oxiline routine add "아침 운동" --at 07:00 --duration 30 \
  --days mon,wed,fri --category health
```

| 옵션 | 설명 |
|---|---|
| `--at <HH:MM>` | 시작 시각 |
| `--duration <MIN>` | 소요 시간 |
| `--days <mon,tue,wed,thu,fri,sat,sun\|weekdays\|weekends\|daily>` | 반복 요일. 콤마 나열 또는 프리셋 |
| `--from <DATE>` / `--until <DATE>` | 한시적 루틴 기간 |
| `--category <ID\|NAME>` | |
| `--notes <TEXT>` | |

### `oxiline export`

```bash
oxiline export --range 2026-07-01:2026-07-31
```

읽기 전용 명령으로, 항상 JSON을 출력한다(`--json` 없어도). 에이전트가 "이번 달 내 하루가 어떻게
구성되어 있는지 분석해줘" 같은 요청을 받았을 때 한 번의 호출로 범위 전체 데이터를 가져가는 용도다.
내부적으로 범위 내 각 날짜에 대해 `get_timeline_for_date`를 호출해 배열로 묶어 반환한다.

### `oxiline doctor`

```bash
$ oxiline doctor
✔ DB 경로: /Users/xxx/Library/Application Support/OxiLine/oxiline.db
✔ 스키마 버전: 4 (최신)
✔ WAL 모드 활성화됨
✔ GUI 프로세스 실행 중 (pid 41213) — 전역 단축키 사용 가능
```
`--json`이면 각 체크 항목을 `{"check": "...", "ok": true, "detail": "..."}` 배열로 반환. 에이전트가
"OxiLine이 정상 설치되어 있는지"를 먼저 확인하는 헬스체크 용도.

## 5.4 에러 출력 형식

`--json` 모드에서 에러는 stdout이 아니라 **stderr**로, 아래 스키마로 출력한다 (stdout은 성공 페이로드
전용으로 깨끗하게 유지 — 에이전트가 stdout만 파싱해도 항상 유효한 JSON이도록 보장하기 위함).

```json
{
  "error": {
    "code": "not_found",
    "message": "ID가 'task_zzzz'인 항목을 찾을 수 없습니다."
  }
}
```

에러 코드 목록: `not_found`, `invalid_argument`, `ambiguous_category`, `db_locked`, `db_migration_failed`,
`permission_denied`, `internal`.

## 5.5 종료 코드

| 코드 | 의미 |
|---|---|
| 0 | 성공 |
| 1 | 일반 오류 (internal) |
| 2 | 사용법 오류 (clap 인자 파싱 실패, invalid_argument) |
| 3 | 대상을 찾을 수 없음 (not_found) |
| 4 | DB 잠금/경합 타임아웃 (db_locked) — 재시도 권장 |
| 5 | 마이그레이션 실패 (db_migration_failed) — `doctor`로 진단 유도 |

## 5.6 향후 확장: MCP 서버 모드 (Phase 3, 스펙만 예비 정의)

Oxi 생태계는 "에이전트에게 도구를 쥐어준다"는 목표를 CLI 호출(서브프로세스 실행)만으로도 달성하지만,
2025~2026년 기준 에이전트-도구 연동의 사실상 표준은 **MCP(Model Context Protocol)**다. CLI 서브프로세스
호출은 모든 에이전트 프레임워크에서 동작하는 최소공배수이므로 v1의 기본 연동 방식으로 유지하되, 다음
확장을 Phase 3 이후 검토한다.

```bash
oxiline mcp serve   # stdio 기반 MCP 서버로 기동, 위 명령어들을 MCP tool로 노출
```

이 모드는 `oxiline-cli` 크레이트에 `rmcp`(공식 Rust MCP SDK) 의존성을 추가해, 기존 core 함수 호출을
그대로 MCP tool 핸들러로 감싸기만 하면 된다 — **core crate는 전혀 수정하지 않는다**는 원칙이 여기서도
유지된다. `oxiline now`, `oxiline task add` 같은 명령이 각각 `oxiline_now`, `oxiline_task_add`라는
MCP tool로 1:1 매핑된다. 이렇게 하면 사용자의 개인 에이전트(`oxi` 등)가 서브프로세스 실행 대신 MCP
클라이언트로 붙어 더 풍부한 스키마(입력 검증, 구조화된 에러)를 얻을 수 있다.

## 5.7 에이전트 사용 시나리오 예시

**시나리오 A — 아침 브리핑 에이전트**: 사용자의 터미널 에이전트가 매일 아침 실행되어
`oxiline today --json`으로 오늘 타임라인을 읽고, 캘린더 앱(별도 도구)과 대조해 충돌을 요약해준다.

**시나리오 B — 회의 후 후속 조치**: 에이전트가 회의록을 요약한 뒤 `oxiline task add "후속 이메일 보내기"
--date tomorrow --at 09:00 --duration 15 --category work`를 호출해 내일 아침 할 일로 자동 등록한다.

**시나리오 C — 하루 마무리 리포트**: `oxiline export --range today:today`로 오늘 완료/미완료 항목을
가져와 회고 요약을 사용자에게 제공한다.

이 세 시나리오 모두 GUI가 실행 중인지 여부와 무관하게 동작해야 한다 — CLI는 GUI에 의존하지 않는다
(§4.5에서 다룬 대로 SQLite 파일이 유일한 공유 지점이다).
