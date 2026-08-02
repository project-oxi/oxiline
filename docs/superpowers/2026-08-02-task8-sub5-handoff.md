# Task 8 Sub5 Handoff — 레거시 코어 삭제 + V5 마이그레이션

> **Status:** Task 8(레거시 전면 철거)의 마지막 꼬리. 앱(제품)은 완전히 recording 패러다임으로 전환됨.
> **Branch:** `main` (origin/main 대비 크게 선행 — 푸시는 사용자 결정)
> **Last commit:** `46a7d58` refactor(app): remove legacy task/routine/timeline/report/cards commands
> **Roadmap:** `docs/superpowers/2026-08-02-task8-legacy-teardown-roadmap.md` (5 서브프로젝트 전체)
> **이전 세션 완료:** Sub1–4 + Sub5-app. **이 문서가 다룰 범위:** Sub5 나머지 = CLI 정리 → 코어 모듈 삭제 → V5 마이그레이션.

---

## 1. 현 위치

### 완료·검증·커밋 (이전 세션)

| Sub | 커밋 | 내용 |
|---|---|---|
| 1 | `523ace2` | `timeline::get_now_context` → `plan::now_summary` (활성 녹화 우선). notifier/tray/CLI `now` 마이그레이션. |
| 2 | `41116af` | CommandPalette ⌘K 하이브리드 (Enter=녹화, `@HH:MM`=plan, free-text=활동 생성, 빈+Enter=정지). |
| 3 | `c2671db` | Header 마커 + HUD OxideBar timeline→records. BlockView 삭제. |
| 4 | `2ede796` | 레거시 4뷰 제거 (Backlog/Week/Report/RoutineManager + tabs/shortcuts). store View/routineManager 제거. |
| 5-app | `4600b84`·`8e52146`·`46a7d58` | dnd backlog/block 분기 + 프론트 hooks/api/types + app 명령·lib.rs 등록 제거. |

**현재 빌드 상태:** 워크스페이스 `cargo build` green, `tsc`/`vitest`/`vite build` green. **앱은 recording 레이어만 호출** (legacy core 참조 0). CLI만 아직 legacy core(tasks/routines/timeline/reports/cards/routine_groups)를 참조하므로 코어 모듈이 존재함.

### 핵심: 의존 순서 (한 패스로 완결)
CLI가 legacy 코어를 참조 → **CLI 정리를 먼저 끝내야 코어 모듈 삭제 가능**. 부분적 CLI 재작성은 비컴파일 상태를 만들므로 CLI 정리→코어 삭제→V5를 한 시퀀스로 끝낼 것.

---

## 2. 남은 작업 (3단계, 순서대로)

### 2.A CLI legacy 하위명령 제거 — `crates/oxiline-cli/`

**legacy Command 변형** (`src/main.rs` run() 디스패치):
- `Command::Today` (≈67–75) — `timeline::get_timeline_for_date`.
- `Command::Task` (≈77–238) — `tasks::*` 인라인 핸들러.
- `Command::Routine` (≈240–342) — `routines::*` 인라인 핸들러.
- `Command::Group` / `handle_group` (≈630) + `resolve_routine_target` (≈612) — **확인 필요**: 디스패치 grep에 `Command::Group`이 안 잡힘 → `grep -n "GroupAction\|handle_group\|Command::Group" crates/oxiline-cli/src/main.rs` 로 매핑 후 제거.

**keeper Command 변형 (건드리지 말 것):** `Now`(59) · `Category`(343) · `Activity`(381) · `Settings`(383) · `Hud`(417) · `Record`(497) · `Plan`(501).

**legacy 액션 enum** (`src/cli.rs`):
- `TaskAction` (≈118–175), `RoutineAction` (≈177–240), `GroupAction` (≈258–282) — 전체 제거.
- keeper enum: `HudAction`, `CategoryAction`, `SettingsAction`, `ActivityAction`, `RecordAction`, `PlanAction`, `MinuteBudget`.

**legacy output 렌더러** (`src/output.rs`): `timeline_text`, `task_text`, routine/report 관련 렌더러. `now_text`/`record_*`/`plan_*`/`activity_*`/`compliance` 렌더러는 keeper. grep `fn .*text\|fn .*_output` 로 분류.

**imports 정리** (`main.rs` 10–13, `output.rs`): `TaskSource`, `TimelineItem` + `tasks, routines, timeline, reports, cards, routine_groups` 제거. keeper: `activities, categories, plan, record, settings, util` + `NowSummary` 등.

### 2.B 코어 모듈 + 타입 삭제 — `crates/oxiline-core/`

**삭제 파일 (6 모듈):**
- `src/tasks.rs`, `src/routines.rs`, `src/reports.rs`, `src/cards.rs`, `src/timeline.rs`, `src/routine_groups.rs`

**삭제 테스트 (3):**
- `tests/cards.rs`, `tests/reports.rs`, `tests/timeline.rs`
- keeper 테스트: `activities.rs`, `plan.rs`, `record.rs`.

**`src/lib.rs` (7–21):** `pub mod`에서 `cards`(8)·`reports`(16)·`routine_groups`(17)·`routines`(18)·`tasks`(20)·`timeline`(21) 제거. keeper: activities/categories/db/error/model/paths/plan/record/settings/util.

**`src/model.rs`:** legacy 모델 타입 제거 — `Task`, `RoutineBlock`, `RoutineStreak`, `WeekReport`, `RangeReport`, `CategoryBreakdown`, `DayTotals`, `TimelineItem`, `CardSuggestion`, `RoutineGroup`, `TaskSource`. keeper: Activity/ActivityInput/Category/Plan/PlanInput/PlanOption/PlanSlot/Record/ActiveSession/RecordState/Compliance/ComplianceState/Scope/NowSummary/NowEntry/SettingsSnapshot. (제거 전 `grep -rn "<TypeName>" crates/oxiline-core/src` 로 잔여 참조 확인 — record.rs/plan.rs가 legacy 타입을 쓰면 안 됨.)

### 2.C V5 마이그레이션 — `crates/oxiline-core/migrations/V5__drop_legacy.sql` (신규)

```sql
-- Task 8 Sub5: drop the legacy task/routine tables (recording layer replaces them).
-- records/activities/plans/plan_options/compliance + settings + categories remain.
-- All legacy FKs are ON DELETE SET NULL; SQLite doesn't FK-check DROP TABLE,
-- but drop referencing tables first for clarity.
DROP TABLE IF EXISTS tasks;            -- source_routine_block_id -> routine_blocks
DROP TABLE IF EXISTS routine_blocks;   -- group_id -> routine_groups, category_id -> categories
DROP TABLE IF EXISTS routine_groups;
```
**스키마 확인 완료** (V1__init.sql): legacy 테이블은 정확히 3개 — `routine_groups`(V1:6)·`routine_blocks`(V1:29)·`tasks`(V1:49). `routine_group_members` 같은 별도 멤버 테이블은 **없음** (루틴↔그룹 매핑은 `routine_blocks.group_id` 컬럼). FK는 모두 `ON DELETE SET NULL`. 인덱스(`idx_routine_blocks_active` V1:46)는 테이블 DROP 시 자동 제거. `db.rs migrations()`가 V5 파일을 자동으로 다음 버전으로 포함(파일명 접두사만 `V5__`).

---

## 3. 검증 계획 (매 커밋 + 최종)

```bash
source $HOME/.cargo/env   # 이 세션에서 PATH가 풀리는 현상 있었음 — 매 셸마다 source

# CLI 정리 후 (2.A):
cargo build -p oxiline-cli           # legacy 코어 참조 사라졌는지
cargo test -p oxiline-cli 2>/dev/null || true

# 코어 삭제 후 (2.B):
cargo build                          # 워크스페이스 전부
cargo test                           # keeper 테스트만 남아 green (activities/plan/record/cards-reports-timeline 제거)

# V5 후 (2.C):
cargo test -p oxiline-core           # 마이그레이션 적용 + 스키마 버전 증가 확인

# 프론트 (변경 없으면 회귀만):
cd crates/oxiline-app && npx tsc --noEmit && npx vitest run && npx vite build
```

**잔여 legacy 심볼 최종 grep (0이어야):**
```
grep -rn "tasks::\|routines::\|timeline::\|reports::\|cards::\|routine_groups::\|TaskSource\|TimelineItem\|RoutineBlock\|get_now_context\|get_timeline" crates --include=*.rs
```

---

## 4. 핵심 코드 앵커 (이것만 먼저 읽을 것)

- `crates/oxiline-cli/src/main.rs:58` — run() 디스패치 시작. legacy/keeper 변형이 나열됨.
- `crates/oxiline-cli/src/cli.rs:38` — `Command` enum. legacy 변형(Today/Task/Routine/Group) + 액션 enum.
- `crates/oxiline-core/src/lib.rs:7` — `pub mod` 목록. 삭제 6개 + keeper.
- `crates/oxiline-core/src/db.rs:17` — `migrations()` (V5 자동 포함).
- `docs/superpowers/2026-08-02-task8-legacy-teardown-roadmap.md` — 전체 로드맵 + 색 시스템 등 핵심 발견.

---

## 5. 위험·주의

| 항목 | 내용 |
|---|---|
| **한 패스 완결** | CLI 재작성은 부분 상태가 비컴파일. 2.A→2.B→2.C를 끊지 말 것 (중간 커밋은 비컴파일 허용하되 한 세션에 완결). |
| **V5 테이블 FK 순서** | 자식→부모 DROP 순서. V1/V2 스키마에서 FK·테이블명 정확히 확인 (위 SQL의 멤버 테이블명은 추정). |
| **`source $HOME/.cargo/env`** | 이전 세션에서 셸마다 cargo PATH가 풀리는 현상. 매 bash 호출에 source. |
| **`record.rs`/`plan.rs` legacy 타입 의존** | 코어 모듈 삭제 전, keeper 모듈(record/plan/activities)이 legacy 타입(TimelineItem 등)을 import 안 하는지 grep 확인. |
| **V5 테이블** | 스키마 확인 완료: legacy 테이블 3개(`tasks`/`routine_blocks`/`routine_groups`), 별도 멤버 테이블 없음. FK 전부 `ON DELETE SET NULL`. |
| **스키마 버전** | V5 추가 후 `db::schema_version` 증가 (`doctor`/테스트가 자동 검증). 기존 사용자 DB에 task/routine 데이터가 있으면 손실 — 본 프로젝트는 dev(main 미푸시)라 수용. |

---

## 6. 권장 첫 커밋 (2.A CLI 정리)

```bash
source $HOME/.cargo/env
# 1. cli.rs: TaskAction/RoutineAction/GroupAction enum + Command의 Today/Task/Routine/Group 변형 제거
# 2. main.rs: run() legacy 분기 + handle_group/resolve_routine_target + imports 정리
# 3. output.rs: timeline_text/task_text/routine·report 렌더러 + imports 정리
# 4. cargo build -p oxiline-cli  (legacy 코어 참조 0 확인)
# 5. git commit -m "refactor(cli): drop legacy task/routine/today/group subcommands (Task 8 sub5)"
```
이후 2.B(코어 삭제), 2.C(V5) 순으로 각각 커밋.

---

## 7. 범위 밖 (본 꼬리에서 안 함)
- origin/main 푸시 (사용자 결정).
- .app 엔드투엔드 smoke (사용자 머신 — followup #2).
- i18n legacy 키(nav.week/backlog/report, backlog.*, routine.*) 정리 — 옵션, 별도.
- CommandPalette 인터랙티브 브라우저 smoke — 권장(followup #2)이나 본 범위 밖.

---

끝. 이 문서 + 로드맵(`…task8-legacy-teardown-roadmap.md`)을 읽으면 Sub5 꼬리(CLI → 코어 삭제 → V5)를 바로 시작할 수 있다.
