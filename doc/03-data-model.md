# 03. Data Model — 엔티티, ERD, SQLite 스키마

## 3.1 설계 목표

- GUI와 CLI가 **완전히 동일한 스키마**를 공유한다 (둘 다 `oxiline-core` 크레이트를 통해서만 접근).
- "루틴은 반복되지만, 특정 날짜의 사정은 다를 수 있다"는 요구를 **가상 occurrence + 지연 구체화
  (lazy materialization)** 패턴으로 해결한다. 별도의 "예외 테이블"을 두지 않고 `tasks` 테이블 하나로
  통합한다 — 스키마를 단순하게 유지하기 위한 핵심 결정이다.
- 모든 시간은 **로컬 타임존 기준 분 단위 정수**로 저장한다 (타임존 동기화, 원격 협업이 없는 로컬
  단일 사용자 앱이므로 UTC 변환의 복잡도를 감수할 이유가 없다). 단, 저장되는 날짜/시각 컬럼은
  아래에 명시된 대로 일관되게 로컬 wall-clock 기준임을 코드 주석에도 명시한다.

## 3.2 핵심 엔티티 개요

```
routine_blocks  ──(생성 시점에 스냅샷)──▶  tasks (source = 'routine')
     │                                         ▲
     │ group_id (nullable)                     │ date 지정 시
     ▼                                         │
routine_groups                            tasks (source = 'manual', date = NULL → 백로그)

categories ──(FK)──▶ routine_blocks
categories ──(FK)──▶ tasks

settings (key-value 단일 테이블)
schema_migrations (rusqlite_migration 관리용)
```

## 3.3 `routine_blocks` — 반복되는 하루의 뼈대

```sql
CREATE TABLE routine_blocks (
    id              TEXT PRIMARY KEY,          -- UUID v7 (시간 정렬 가능)
    group_id        TEXT REFERENCES routine_groups(id) ON DELETE SET NULL,
    title           TEXT NOT NULL,
    category_id     TEXT REFERENCES categories(id) ON DELETE SET NULL,
    start_minute    INTEGER NOT NULL CHECK (start_minute BETWEEN 0 AND 1439),
    duration_minute INTEGER NOT NULL CHECK (duration_minute BETWEEN 1 AND 1440),
    weekday_mask    INTEGER NOT NULL,           -- bit0=월 ... bit6=일. 0b1111111 = 매일
    effective_from  TEXT,                       -- ISO date, NULL이면 무기한 시작
    effective_until TEXT,                       -- ISO date, NULL이면 무기한 지속
    is_active       INTEGER NOT NULL DEFAULT 1, -- 0이면 일시 정지(휴가 모드 등)
    color_override  TEXT,                       -- OKLCH 문자열, NULL이면 category 색 사용
    notes           TEXT,
    sort_order      INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL,              -- ISO 8601 UTC
    updated_at      TEXT NOT NULL
);
CREATE INDEX idx_routine_blocks_active ON routine_blocks(is_active);
```

- `weekday_mask`: 예) 평일만 = `0b0011111` = 31, 매일 = 127, 화/목만 = `0b0001010` = 10.
- `effective_from` / `effective_until`은 "이번 학기 동안만", "다이어트 3개월 루틴" 같은 한시적 루틴을
  위한 것이다. 둘 다 NULL이면 영구 루틴.

## 3.4 `routine_groups` — 루틴 묶음 (Phase 2, 스키마는 v1부터 존재)

```sql
CREATE TABLE routine_groups (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,           -- 예) "평일 아침 루틴", "주말 루틴"
    icon        TEXT,                    -- lucide 아이콘 이름
    is_active   INTEGER NOT NULL DEFAULT 1,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
```

MVP에서는 UI로 그룹 생성 기능을 노출하지 않아도 되지만(대신 개별 `routine_blocks`만 다룸), 스키마와
"그룹 전체 on/off" 로직은 core crate에 처음부터 넣어 나중에 UI만 얹으면 되게 한다.

## 3.5 `tasks` — 실제로 존재하는 하루의 항목 (수동 + 구체화된 루틴 occurrence)

```sql
CREATE TABLE tasks (
    id                      TEXT PRIMARY KEY,
    date                    TEXT,              -- ISO date (YYYY-MM-DD). NULL = 백로그(날짜 없음)
    title                   TEXT NOT NULL,
    category_id             TEXT REFERENCES categories(id) ON DELETE SET NULL,
    start_minute            INTEGER,           -- NULL = 시간 미지정 (백로그/할일만)
    duration_minute         INTEGER,
    is_done                 INTEGER NOT NULL DEFAULT 0,
    done_at                 TEXT,
    is_skipped              INTEGER NOT NULL DEFAULT 0, -- 루틴 occurrence를 "오늘만 건너뛰기"
    source                  TEXT NOT NULL CHECK (source IN ('manual','routine')),
    source_routine_block_id TEXT REFERENCES routine_blocks(id) ON DELETE SET NULL,
    notes                   TEXT,
    sort_order              INTEGER NOT NULL DEFAULT 0,
    created_at              TEXT NOT NULL,
    updated_at              TEXT NOT NULL
);
CREATE INDEX idx_tasks_date ON tasks(date);
CREATE INDEX idx_tasks_routine_origin ON tasks(source_routine_block_id, date);
CREATE UNIQUE INDEX uq_tasks_materialized_occurrence
    ON tasks(source_routine_block_id, date)
    WHERE source_routine_block_id IS NOT NULL;
```

- `source='manual'` + `date=NULL` → 백로그(Inbox) 항목.
- `source='manual'` + `date` 있음 → 특정 날짜의 일회성 할일.
- `source='routine'` → 어떤 `routine_block`이 특정 `date`에 대해 **구체화된** 행. 사용자가 완료 체크,
  시간 변경, 건너뛰기, 삭제 중 하나라도 하면 이 행이 생성된다 (§3.7 참고).

## 3.6 `categories` — 색상 태그

```sql
CREATE TABLE categories (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    color_hue   REAL NOT NULL,      -- OKLCH H 값 (0-360). L/C는 디자인 토큰에서 전역 관리
    icon        TEXT,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    is_builtin  INTEGER NOT NULL DEFAULT 0,  -- 시드 데이터 여부 (삭제 방지 UI 힌트용)
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
```

시드 데이터(마이그레이션 시 삽입, `06-design-system.md`의 카테고리 팔레트와 1:1 대응):

| name | color_hue | icon |
|---|---|---|
| 업무 (Work) | 250 | `briefcase` |
| 건강 (Health) | 145 | `heart-pulse` |
| 학습 (Study) | 300 | `book-open` |
| 휴식 (Rest) | 350 | `moon` |
| 개인 (Personal) | 90 | `user` |
| 기타 (Other) | 250 (무채색 처리, C=0) | `circle` |

> 참고: 시맨틱 색상인 `signal-rust`(경고/지연)와 `accent-oxide`(브랜드/지금선)에 쓰이는 hue(35, 189)는
> 사용자 카테고리 팔레트에서 제외한다 — 의미 충돌을 막기 위함 (`06-design-system.md` §6.2 참고).

## 3.7 가상 Occurrence와 구체화(materialize) 전략

특정 날짜 `D`의 타임라인을 렌더링할 때, 화면에 보여줄 항목 = **(A) 가상 occurrence** ∪ **(B) 구체화된
tasks**.

- **(A) 가상 occurrence**: `is_active=1`이고 `weekday_mask`에 `D`의 요일 비트가 켜져 있고
  (`effective_from`/`effective_until` 범위 안에 `D`가 있는 경우) `routine_blocks` 각각에 대해,
  `tasks`에 `(source_routine_block_id=block.id, date=D)` 조합의 행이 **아직 없으면** 가상으로
  계산해서 보여준다. 이 항목은 DB에 존재하지 않으므로 id는 `virtual:{block_id}:{date}` 같은 합성
  키로 프론트엔드에서만 취급한다.
- **(B) 구체화**: 사용자가 가상 occurrence에 대해 다음 중 하나를 수행하는 순간, `tasks`에 실제 행을
  INSERT한다 (core crate의 `materialize_occurrence()` 함수 하나로 통일):
  - 완료 체크 (`is_done=1, done_at=now`)
  - 오늘만 건너뛰기 (`is_skipped=1`)
  - 오늘만 시간/제목 변경 (해당 필드만 덮어씀, `source='routine'` 유지)
  - 오늘만 삭제 ("이 발생만 삭제"는 구체화 후 다시 삭제하는 게 아니라, 삭제 의도를 표시하는
    `is_skipped=1`과 동일하게 취급 — 별도의 "deleted" 상태를 만들지 않아 상태 공간을 단순하게 유지)
- 루틴 자체를 수정(`routine_blocks` 갱신)해도 **과거에 이미 구체화된 tasks 행은 건드리지 않는다** —
  구체화는 "그 날짜의 스냅샷"이라는 원칙을 지킨다.

이 전략의 장점: 몇 달치 루틴 occurrence를 미리 다 생성해둘 필요가 없어 DB가 가볍고, 스키마도
"예외 테이블" 없이 하나로 끝난다. 단점: 조회 시 항상 "활성 routine_blocks + 해당 날짜의 tasks"를
머지하는 로직이 필요하다 → 이 로직은 **반드시 `oxiline-core`에 `get_timeline_for_date(date) -> Vec<TimelineItem>`
단 하나의 함수로 캡슐화**하고, GUI 커맨드와 CLI 서브커맨드 모두 이 함수만 호출한다 (다른 곳에서
직접 SQL을 짜지 않는다).

## 3.8 `settings` — 키-값 설정

```sql
CREATE TABLE settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,   -- JSON 문자열로 저장 (타입은 애플리케이션 레이어에서 강제)
    updated_at  TEXT NOT NULL
);
```

초기 시드 키:

| key | 기본값 | 설명 |
|---|---|---|
| `locale` | `"system"` | `ko` \| `en` \| `system` |
| `theme` | `"system"` | `light` \| `dark` \| `system` |
| `global_hotkey` | `"CmdOrCtrl+Shift+O"` | 전역 단축키 조합 |
| `hud_duration_ms` | `2000` | 플로팅 HUD 자동 소멸 시간 |
| `day_start_hour` | `5` | 타임라인 렌더링 시작 시각 |
| `day_end_hour` | `26` | 타임라인 렌더링 종료 시각 (26 = 다음날 새벽 2시까지 표시) |
| `week_starts_on` | `"mon"` | 주 시작 요일 |
| `launch_at_login` | `true` | 로그인 시 자동 실행 |
| `workload_warning_minutes` | `600` | 하루 계획 총합 경고 임계값(분), 0이면 비활성화 |
| `schema_version` | (자동 관리) | 마이그레이션 버전 (rusqlite_migration이 별도 테이블로도 관리하므로 중복 안전장치) |

## 3.9 PRAGMA 및 동시성 설정

GUI와 CLI가 같은 파일을 동시에 여닫을 수 있으므로 연결 생성 시 항상 아래를 적용한다
(`oxiline-core::db::open()` 내부에 하드코딩):

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
```

WAL 모드는 리더(읽기)와 라이터(쓰기)가 서로를 거의 막지 않게 해주므로, "CLI가 쓰는 순간 GUI가 멈추는"
문제를 피할 수 있다. `busy_timeout`은 드물게 발생하는 쓰기 경합 시 즉시 에러를 던지지 않고 최대 5초
재시도하게 한다.

## 3.10 마이그레이션 전략

- `rusqlite_migration` 크레이트로 순차 `up` SQL 스크립트를 관리한다 (`oxiline-core/migrations/*.sql`).
- **GUI와 CLI 둘 다 시작 시 동일한 `oxiline_core::db::open_and_migrate(path)`를 호출**한다 — 마이그레이션
  로직이 두 곳에 중복되면 스키마 drift가 생기므로, 반드시 core crate 하나에만 존재해야 한다.
- 최초 실행(파일이 없을 때) 시 `categories` 시드 데이터도 같은 마이그레이션 파일에서 INSERT한다.

## 3.11 Rust 도메인 타입 스케치 (core crate)

```rust
// oxiline-core/src/model.rs
use serde::{Serialize, Deserialize};
use specta::Type;

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct RoutineBlock {
    pub id: String,
    pub group_id: Option<String>,
    pub title: String,
    pub category_id: Option<String>,
    pub start_minute: u16,
    pub duration_minute: u16,
    pub weekday_mask: u8,
    pub effective_from: Option<String>,
    pub effective_until: Option<String>,
    pub is_active: bool,
    pub color_override: Option<String>,
    pub notes: Option<String>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct Task {
    pub id: String,
    pub date: Option<String>,
    pub title: String,
    pub category_id: Option<String>,
    pub start_minute: Option<u16>,
    pub duration_minute: Option<u16>,
    pub is_done: bool,
    pub is_skipped: bool,
    pub source: TaskSource,
    pub source_routine_block_id: Option<String>,
    pub notes: Option<String>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskSource { Manual, Routine }

/// get_timeline_for_date()가 반환하는, 프론트엔드가 실제로 그리는 통합 뷰 모델.
/// 가상 occurrence와 구체화된 task를 동일한 셰이프로 노출해 프론트엔드가 구분할 필요 없게 한다.
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct TimelineItem {
    pub id: String,               // 실제 task.id 또는 "virtual:{block_id}:{date}"
    pub is_virtual: bool,
    pub title: String,
    pub start_minute: Option<u16>,
    pub duration_minute: Option<u16>,
    pub category_id: Option<String>,
    pub is_done: bool,
    pub is_skipped: bool,
    pub origin_routine_block_id: Option<String>,
}
```

`specta::Type`을 파생시켜 두면 `04-architecture.md`에서 다루는 tauri-specta가 이 구조체를 그대로
TypeScript 타입으로 변환해 프론트엔드와 100% 동기화된 타입을 자동 생성한다.
