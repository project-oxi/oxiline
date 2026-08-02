# 다중선택 드롭-병합 sort_order 경쟁 수정 — 벌크 `add_plan_options` 설계

> 날짜: 2026-08-02
> 범위: 기술부채. 다중 활동을 기존 계획 카드에 드롭할 때 N개의 단일 `add_plan_option`이 개별 풀 커넥션으로 발화하여 `sort_order` 경쟁이 발생하는 문제를, **`BEGIN IMMEDIATE` 단일 트랜잭션 벌크 명령**으로 원천 해소한다.
> 선행: HUD·날짜 popover·OR-계획 다중선택+병합+리사이즈 3기능 완료(`939f8d2`). 본 설계는 `2026-08-02-hud-date-orplan-followup.md` 항목 4(parked)의 실행이다.
> 방침: **완전 교체** — 단일 `add_option`/`add_plan_option`/`useAddPlanOption` 경로를 제거하고 벌크 경로로 대체한다(dead code 제거 + 경쟁 표면 원천 차단).

## 0. 현황과 문제

`dnd.tsx` `handleDragEnd`의 드롭-병합 분기(dnd.tsx:84)는 다중 활동을 기존 계획에 추가할 때:
```ts
activityIds.forEach((aid) => addOption.mutate({ planId, activityId: aid }));
```
각 `mutate`가 별도 `invoke` → 별도 풀 커넥션(`AppState`는 r2d2 풀, max 8; `state.conn()`이 호출마다 별도 커넥션 반환). 이 풀은 동시 GUI+CLI 접근을 위해 설계된 것(db.rs:26)이라 다중 writer는 설계된 시나리오다.

코어 `plan::add_option`(plan.rs:217)은 트랜잭션조차 없이 비원자 read-then-write:
```rust
let next_order = SELECT COALESCE(MAX(sort_order),-1)+1 ...;  // read (쓰기락 없음)
conn.execute(INSERT ... next_order);                          // write
```
WAL은 읽기를 차단하지 않으므로, 동시 커넥션들이 같은 `MAX`를 읽고 같은 `sort_order`로 INSERT. `plan_options`에 `(plan_id, sort_order)` UNIQUE 제약이 없어 INSERT는 성공하지만 표시 순서만 비결정적(데이터 손상·기능 장애 无; `→실행` 해석은 `activity_id` 사용).

**영향**: 표시 순서 비결정적. 머지는 안 막힌다(후속 문서 판단과 일치). 그러나 경쟁 표면을 남기는 것은 기술부채.

## 1. 해법 — `BEGIN IMMEDIATE` 단일 트랜잭션 벌크 추가

### 1.0 왜 DEFERRED 트랜잭션으로는 부족한가 (핵심)

경쟁의 본질은 "read-then-compute-write가 커넥션 간 원자가 아닌 것"이다. 따라서 **read 시점에 쓰기락을 이미 잡고 있어야** 한다.

- **DEFERRED**(`unchecked_transaction()`의 기본 동작 = `Transaction::new_unchecked(conn, Deferred)`): `BEGIN`은 어떤 락도 잡지 않는다. 첫 `SELECT MAX`는 쓰기락 없이 WAL 스냅샷으로 읽힌다 → 두 풀 커넥션이 commit 전 같은 `MAX`를 읽고, 이후 `busy_timeout`이 쓰기를 직렬화하더라도 **둘 다 같은 sort_order를 계산해 INSERT** → 경쟁 잔존. (WAL에서는 read-then-write 패턴이 `SQLITE_BUSY_SNAPSHOT`을 낼 수도 있다.) 단순히 "트랜잭션으로 감싸면 해결된다"는 주장은 거짓이다.
- **IMMEDIATE**(`Transaction::new_unchecked(conn, TransactionBehavior::Immediate)` → `BEGIN IMMEDIATE`): `BEGIN` 즉시 RESERVED(쓰기)락을 선점한다. 한 번에 한 커넥션만 보유, 나머지는 `busy_timeout=5000`으로 commit까지 대기. 따라서 `SELECT MAX`가 **쓰기락을 잡은 상태**로 최신 committed 상태를 읽고, read→commit 사이에 다른 writer가 끼어들 수 없다 → sort_order 단조·유일 구조적 보장.

> **`update_plan` 함정 주의**: `update_plan`(plan.rs:146)은 DEFERRED 트랜잭션에서 안전해 보이지만, 이는 DEFERRED 덕분이 **아니다**. `update_plan`은 blind `DELETE` + `INSERT … sort_order = idx`(루프 인덱스)로, **현재 DB 상태를 읽어 계산에 쓰지 않는다**(stale-read 의존성 없음). 반면 `add_options`는 `MAX`+기존 옵션을 읽어 sort_order를 계산하므로 stale-read 위험에 직면한다. 그래서 `update_plan`의 DEFERRED 패턴을 `add_options`에 그대로 복사하면 경쟁이 남는다. **read-then-compute에는 IMMEDIATE가 필수.**

### 1.1 코어 `plan::add_options` (plan.rs, `add_option` 대체)

```rust
/// Append OR alternatives to a plan in ONE `BEGIN IMMEDIATE` transaction. The
/// write lock is acquired at `BEGIN`, BEFORE the `MAX(sort_order)` read, so the
/// read sees the latest committed state and no other pooled connection can
/// interleave a write between the read and the commit. `activity_ids` keep
/// their input order; already-linked activities (and repeats within the input)
/// are skipped — the returned `Vec<PlanOption>` holds one row per *unique
/// input* in input order (existing-or-new). `sort_order` continues from
/// `MAX + 1`, assigned monotonically inside the locked transaction. Empty
/// input short-circuits to an empty `Vec` without touching the DB.
pub fn add_options(conn: &Connection, plan_id: &str, activity_ids: &[String]) -> Result<Vec<PlanOption>>;
```

알고리즘:
1. `activity_ids.is_empty()` → `return Ok(vec![])` (DB 미접근).
2. `get_plan(conn, plan_id)?` — NotFound 가드(기존 `add_option`과 동일).
3. `let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;` — **`BEGIN IMMEDIATE`로 쓰기락 선점** (`&Connection` 호환; `unchecked_transaction()`은 이것의 Deferred 래퍼). 임포트: `use rusqlite::{Connection, Transaction, TransactionBehavior, params};`.
4. 기존 옵션 1회 로드(쓰기락 보유 상태): `SELECT * FROM plan_options WHERE plan_id = ?1 ORDER BY sort_order` → `Vec<PlanOption>`(`row_from_option` 재사용).
   - `existing: HashMap<&activity_id, PlanOption>` 구성.
   - `next_order = existing rows의 sort_order 최댓값 + 1` (없으면 0).
5. 입력 순회(순서 보존), `seen: HashSet`로 입력 내 중복은 첫 등장만:
   - `existing`에 있으면 → 해당 `PlanOption`을 결과에 push (INSERT 없음).
   - 아니면 → `id = util::new_id()`, `tx.execute(INSERT … next_order)`, `PlanOption{id, plan_id, activity_id, sort_order: next_order}`를 **직접 construct**하여 push (재조회 无 — `PlanOption`은 테이블 4컬럼과 정확히 일치, 타임스탬프 없음), `next_order += 1`.
6. `tx.commit()?` — 쓰기락 해제.
7. 결과 `Vec<PlanOption>` 반환(입력 순, dedup, existing-or-new).

**접근법 선택 — IMMEDIATE Rust 루프 vs 단일 `INSERT…SELECT json_each`**: 단일 SQL문도 실행 중 쓰기락을 잡아 원자적이지만, (a) `id` 생성이 프로젝트 UUID v7 규약(`util::new_id`)을 SQL `randomblob`로 대체해야 하고, (b) 입력 순서를 보존하며 within-input dedup를 SQL로 표현하기 어렵다. Rust 루프가 id 규약·dedup·순서를 자연스럽게 처리하고 IMMEDIATE로 정확성을 확보하므로 **Rust 루프**를 선택한다.

**우아함 근거**:
- `BEGIN IMMEDIATE`가 read 시점에 쓰기락을 잡아 read-then-compute-write를 진정 원자화 → 경쟁 **구조적** 제거(`busy_timeout` 직렬화에 기대지 않음; DEFERRED의 stale-read 함정 회피).
- 기존 옵션 조회 1회 + 신규 1행당 INSERT 1회 — 쿼리 최소.
- `PlanOption` 직접 construct — 행별 재조회 제거.
- 입력 순서 보존 + dedup — 호출측 멘탈 모델 단순.

`add_option`(단일)은 제거 — 유일 소비자가 교체 대상 명령이므로 dead code. (단일은 트랜잭션 자체가 없어 경쟁이 더 심했다.)

### 1.2 app 명령 (commands.rs + lib.rs)

```rust
#[tauri::command]
#[specta::specta]
pub fn add_plan_options(
    state: State<AppState>,
    plan_id: String,
    activity_ids: Vec<String>,
) -> Result<Vec<PlanOption>, String> {
    plan::add_options(&state.conn(), &plan_id, &activity_ids).map_err(map_err)
}
```
- `add_plan_option`(단일) 제거.
- `lib.rs` `collect_commands!`: `commands::add_plan_option` → `commands::add_plan_options`.
- 인자는 Tauri JS 바인딩 컨벤션(camelCase) 준수: `planId`, `activityIds`.

### 1.3 프론트 (api.ts / hooks.ts / dnd.tsx)

- `api.addPlanOptions(planId, activityIds: string[])` → `invoke<PlanOption[]>("add_plan_options", { planId, activityIds })`. `addPlanOption` 제거.
- `useAddPlanOptions()`: `mutationFn: ({planId, activityIds}) => api.addPlanOptions(planId, activityIds)`, `onSuccess` → `qc.invalidateQueries({queryKey:["slots"]})` + `["plans"]`(기존 `useAddPlanOption`과 동일). `useAddPlanOption` 제거.
- `dnd.tsx` 드롭-병합 분기:
  ```ts
  // before
  activityIds.forEach((aid) => addOption.mutate({ planId, activityId: aid }));
  // after
  addOptions.mutate({ planId, activityIds });
  ```
  import: `useAddPlanOption` → `useAddPlanOptions`.

## 2. 오류 처리

- 빈 입력 → 빈 Vec 즉시 반환(NO-OP, NotFound 아님, DB 미접근).
- plan 미존재 → `NotFound`(`get_plan`).
- 존재하지 않는 activity_id → FK 제약이 INSERT 단계에서 실패 → 트랜잭션 전체 롤백(`Transaction` drop → rollback) → `CoreError` → `map_err`로 프론트에 문자열 전파(기존 `add_option`과 동일). 부분 추가 없음(원자성).

## 3. 검증 — 코어 통합 테스트 (tests/plan.rs 신규)

`db()` 헬퍼 재사용(기존 plan 테스트와 동일). 1–4는 단일 커넥션 단위 로직, 5는 **실제 동시성 스트레스**(이 작업이 존재하는 이유).

1. **단조·유일 + 기존 dedup**: plan(옵션 a1, order 0)에서 `add_options(&[a2, a3, a1])`. 검증 두 축: (a) **반환 Vec**는 입력 순서 [a2(order 1, 신규), a3(order 2, 신규), a1(order 0, 기존)] — existing-or-new per input; (b) **DB 전체 옵션 집합**(`SELECT … ORDER BY sort_order`)은 [a1:0, a2:1, a3:2]로 sort_order 단조 증가·유일, a1 1행(미중복).
2. **입력 내 중복**: `add_options(&[a4, a4, a5])` → a4 1회(다음 order), a5 그 다음; 결과 길이 2(입력 내 중복 제거), DB에도 각 1행.
3. **빈 입력**: `add_options(&[])` → 빈 Vec, plan_options 행수 변동 없음.
4. **NotFound**: 존재하지 않는 plan_id → `Err(NotFound)`.

**5. 동시성 스트레스 (경쟁 판별 테스트)** — 단일 커넥션 테스트가 구조적으로 잡을 수 없는, 본 작업의 핵심 검증:
- 셋업: `NamedTempFile` 1개. `open_and_migrate`로 setup 커넥션 1개(마이그레이션 + WAL/busy_timeout=5000 적용 — db.rs:28-33, 43), `ensure_defaults`, 활동 + plan(기존 옵션 a0, order 0) 생성.
- 스레드 `T=4`, 각각 `open_and_migrate(path)`로 **자기 전용 커넥션**(r2d2 풀 시나리오 재현; 마이그레이션은 이미 적용돼 no-op, busy_timeout/WAL은 매 커넥션에 설정).
- 각 스레드가 서로 다른 `K=25`개 활동을 **한 번에 1개씩** `add_options(&conn, plan, &[aid])`로 같은 plan에 병렬 추가(MAX read-then-write 경쟁 최대 가압).
- 단언: (a) **에러 제로** — 모든 호출 `Ok`(IMMEDIATE + busy_timeout=5000이 쓰기락을 대기시키므로 `SQLITE_BUSY`/`SQLITE_BUSY_SNAPSHOT` 없음); (b) **sort_order 전역 유일** — 최종 `SELECT sort_order FROM plan_options WHERE plan_id=? ORDER BY sort_order`가 `1 + T·K`개이고 모두 distinct.
- **판별력**: DEFERRED였으면 (a) duplicate sort_order 또는 (b) `SQLITE_BUSY_SNAPSHOT` 에러로 실패. 즉 이 테스트는 IMMEDIATE 정확성을 **경험적으로** 정착한다(구조적 보장 §1.0만으로는 순환논증).

> 단일 드롭(1개 추가) 케이스는 DEFERRED/IMMEDIATE 모두 정상 — 본 테스트가 표적하는 것은 풀이 설계된 **동시** 시나리오다.

## 4. 명시적 범위 밖 (YAGNI)

- **`(plan_id, sort_order)` UNIQUE 제약 추가**: IMMEDIATE 트랜잭션이 경쟁을 원천 차단하므로 불필요. 추가하면 마이그레이션(V5) + 기존 중복 행(있다면) 마이그레이션 부담 — 정당화 안 됨.
- **단일 추가 경로 유지**: 단일 추가가 필요하면 `add_plan_options(plan_id, [one])`로 표현. dead code 제거 원칙.
- **UI 변경·새 기능**: 없음. 호출부 1회 호출로 교체가 전부.
- **N+1 INSERT 최적화**(단일 multi-row INSERT): N은 사용자 드래그 선택 수(수 단위)로 병목이 아니며, IMMEDIATE 트랜잭션 내 순차 INSERT가 충분.

## 5. 영향 받는 파일

- 코어: `crates/oxiline-core/src/plan.rs`(`add_options` 추가 + 임포트 `Transaction`/`TransactionBehavior`, `add_option` 제거), `crates/oxiline-core/tests/plan.rs`(테스트 5건 — 단일 4 + 동시성 1).
- app Rust: `src-tauri/src/commands.rs`(`add_plan_options` 추가, `add_plan_option` 제거), `src-tauri/src/lib.rs`(`collect_commands!` 등록 교체).
- 프론트: `src/lib/api.ts`(`addPlanOptions` 추가, `addPlanOption` 제거), `src/hooks.ts`(`useAddPlanOptions` 추가, `useAddPlanOption` 제거), `src/lib/dnd.tsx`(호출부 교체).
- 모델/타입/스키마: 변경 없음(`PlanOption` 재사용, 마이그레이션 없음).

## 6. 위험

- **동시성 모델 함정(핵심)**: DEFERRED 트랜잭션(`unchecked_transaction()`)으로 감싸면 "트랜잭션이 원자화한다"는 착각으로 경쟁이 잔존한다(§1.0). `BEGIN IMMEDIATE`(`Transaction::new_unchecked(conn, Immediate)`)가 read 시점 쓰기락 선점의 필수 조건. `update_plan`의 DEFERRED 패턴은 blind-write라 안전한 것이지 본 사례의 모델이 아님. — 테스트 5가 이를 경험적으로 방어.
- **검증 신뢰도**: 단일 커넥션 단위 테스트(1–4)는 경쟁을 잡지 못한다. "구조적 보장"에만 의존하면 순환논증이 되므로, 반드시 다중 커넥션 병렬 스트레스(5)로 경험적 정착이 병행되어야 한다.
- **specta 바인딩 재생성**: `#[cfg(debug_assertions)]` dev 빌드에서 `bindings.ts` 재export. `tsc`/`vite`로 타입 정합성 확인.
- **잔여 단일 경로 참조**: `addPlanOption`/`useAddPlanOption`/`add_plan_option`/`add_option`의 다른 참조가 없음을 grep으로 사전 확인(탐색 단계에서 유일 소비자 dnd.tsx:84임 확인됨). 제거 시 누락 참조가 컴파일/타입 에러로 즉시 노출.
- **FK 롤백**: 입력에 섞인 미존재 activity_id가 전체 배치를 롤백 → 사용자는 아무 옵션도 추가되지 않은 상태로 에러를 봄. 기존 단일 동작과 일관(단일에서도 FK 실패 시 추가 안 됨).
