# 벌크 `add_plan_options` (sort_order 경쟁 수정) 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 다중 활동 드롭-병합의 `sort_order` 경쟁을 `BEGIN IMMEDIATE` 단일 트랜잭션 벌크 명령으로 원천 해소하고, 단일 옵션 경로를 완전 제거한다.

**Architecture:** 코어 `plan::add_options(conn, plan_id, &[activity_ids])` 가 `BEGIN IMMEDIATE`(`Transaction::new_unchecked(conn, Immediate)`)로 쓰기락을 read 전에 선점해 `MAX(sort_order)` read-then-compute-write를 원자화. app 명령 `add_plan_options` → 프론트 `addPlanOptions`/`useAddPlanOptions` → `dnd.tsx` 1회 호출. 단일 `add_option`/`add_plan_option`/`useAddPlanOption`/`find_option` dead code 제거.

**Tech Stack:** Rust(rusqlite 0.32.1, `TransactionBehavior::Immediate`), Tauri+specta 명령, React Query mutation, TypeScript.

**Spec:** `docs/superpowers/specs/2026-08-02-bulk-plan-options-design.md`

## Global Constraints

- 코어 트랜잭션은 **반드시 `BEGIN IMMEDIATE`** (`Transaction::new_unchecked(conn, TransactionBehavior::Immediate)`). DEFERRED(`unchecked_transaction()`)는 금지 — read 시점 쓰기락이 없어 경쟁 잔존(spec §1.0).
- 코어 fn은 `&Connection`(`&mut` 아님) — 기존 `update_plan`/`add_option`과 동일 제약.
- 명령 인자는 Tauri JS 바인딩 camelCase: `planId`, `activityIds`.
- `PlanOption` 타입/스키마 변경 없음(`{id, plan_id, activity_id, sort_order}`, 마이그레이션 없음).
- 단일 경로(`add_option`, `add_plan_option`, `useAddPlanOption`, `find_option`)는 **제거** — dead code.

---

### Task 1: 코어 `add_options` + 단일 옵션 제거 + 단일 커넥션 테스트

**Files:**
- Modify: `crates/oxiline-core/src/plan.rs`(임포트 L19, `add_option` L215-242 → `add_options`, `find_option` L244-268 제거)
- Test: `crates/oxiline-core/tests/plan.rs`(테스트 4건 추가)

**Interfaces:**
- Consumes: `get_plan(conn, id) -> Result<Plan>`(plan.rs:140), `row_from_option(&Row) -> rusqlite::Result<PlanOption>`(plan.rs:36), `util::new_id() -> String`, `CoreError`/`Result`.
- Produces: `pub fn add_options(conn: &Connection, plan_id: &str, activity_ids: &[String]) -> Result<Vec<PlanOption>>`.

- [ ] **Step 1: 임포트에 `Transaction`, `TransactionBehavior` 추가**

`plan.rs:19`:
```rust
// before
use rusqlite::{Connection, params};
// after
use rusqlite::{Connection, Transaction, TransactionBehavior, params};
```

- [ ] **Step 2: `add_option` + `find_option` 제거, `add_options` 추가**

`plan.rs:214-268`(`add_option` doc+본체 + `find_option` doc+본체)를 아래로 교체. (`find_option`은 `add_option`만 쓰므로 함께 dead code 제거 — 교체 전 `grep find_option`으로 추가 사용처 없음 재확인.)
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
pub fn add_options(
    conn: &Connection,
    plan_id: &str,
    activity_ids: &[String],
) -> Result<Vec<PlanOption>> {
    if activity_ids.is_empty() {
        return Ok(Vec::new());
    }
    // Surface NotFound for unknown plans before opening a transaction.
    let _ = get_plan(conn, plan_id)?;
    // BEGIN IMMEDIATE: RESERVED (write) lock at `BEGIN`, BEFORE the MAX read.
    // (unchecked_transaction() is the Deferred flavor and would race.)
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    let existing: Vec<PlanOption> = tx
        .prepare("SELECT * FROM plan_options WHERE plan_id = ?1 ORDER BY sort_order")?
        .query_map(params![plan_id], row_from_option)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut by_activity: std::collections::HashMap<&str, PlanOption> =
        std::collections::HashMap::new();
    for opt in &existing {
        by_activity.insert(opt.activity_id.as_str(), opt.clone());
    }
    // existing is ordered by sort_order ASC → last is the max.
    let mut next_order: i32 = existing.last().map(|o| o.sort_order + 1).unwrap_or(0);
    let mut out: Vec<PlanOption> = Vec::with_capacity(activity_ids.len());
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for aid in activity_ids {
        let aid_str = aid.as_str();
        if !seen.insert(aid_str) {
            continue; // duplicate within input — keep first occurrence
        }
        if let Some(existing_opt) = by_activity.get(aid_str) {
            out.push(existing_opt.clone());
            continue; // already an option — no INSERT
        }
        let id = util::new_id();
        tx.execute(
            "INSERT INTO plan_options (id, plan_id, activity_id, sort_order)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, plan_id, aid_str, next_order],
        )?;
        out.push(PlanOption {
            id,
            plan_id: plan_id.to_string(),
            activity_id: aid_str.to_string(),
            sort_order: next_order,
        });
        next_order += 1;
    }
    tx.commit()?;
    Ok(out)
}
```

- [ ] **Step 3: 단일 커넥션 테스트 4건 추가** (`tests/plan.rs` 말단)

```rust
fn sort_orders(c: &Connection, plan_id: &str) -> Vec<i32> {
    c.prepare("SELECT sort_order FROM plan_options WHERE plan_id = ?1 ORDER BY sort_order")
        .unwrap()
        .query_map(rusqlite::params![plan_id], |r| r.get::<_, i32>(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect()
}

#[test]
fn add_options_monotonic_unique_and_dedups_existing() {
    let (_f, c) = db();
    let a1 = mk_activity(&c, "a1");
    let a2 = mk_activity(&c, "a2");
    let a3 = mk_activity(&c, "a3");
    let p = oxiline_core::plan::create_plan(
        &c,
        oxiline_core::model::PlanInput {
            date: None,
            start_minute: 9 * 60,
            duration_minute: 60,
            weekday_mask: 0b0000001,
            title: None,
            activity_ids: vec![a1.id.clone()],
        },
    )
    .unwrap();
    // a1 already an option (order 0); a2/a3 new.
    let out = oxiline_core::plan::add_options(&c, &p.id, &[a2.id.clone(), a3.id.clone(), a1.id.clone()]).unwrap();
    // (a) return: input order, existing-or-new, one row per unique input
    assert_eq!(out.len(), 3);
    assert_eq!(out[0].activity_id, a2.id); assert_eq!(out[0].sort_order, 1);
    assert_eq!(out[1].activity_id, a3.id); assert_eq!(out[1].sort_order, 2);
    assert_eq!(out[2].activity_id, a1.id); assert_eq!(out[2].sort_order, 0);
    // (b) DB set: monotonic, unique, a1 single row
    let orders = sort_orders(&c, &p.id);
    assert_eq!(orders, vec![0, 1, 2]);
}

#[test]
fn add_options_dedups_within_input() {
    let (_f, c) = db();
    let a4 = mk_activity(&c, "a4");
    let a5 = mk_activity(&c, "a5");
    let p = oxiline_core::plan::create_plan(
        &c,
        oxiline_core::model::PlanInput {
            date: None, start_minute: 9 * 60, duration_minute: 60,
            weekday_mask: 0b0000001, title: None, activity_ids: vec![a4.id.clone()],
        },
    ).unwrap(); // a4 = order 0
    let out = oxiline_core::plan::add_options(&c, &p.id, &[a4.id.clone(), a4.id.clone(), a5.id.clone()]).unwrap();
    assert_eq!(out.len(), 2); // within-input dup collapsed
    assert_eq!(out[0].activity_id, a4.id); assert_eq!(out[0].sort_order, 0); // existing
    assert_eq!(out[1].activity_id, a5.id); assert_eq!(out[1].sort_order, 1); // new
    assert_eq!(sort_orders(&c, &p.id), vec![0, 1]);
}

#[test]
fn add_options_empty_is_noop() {
    let (_f, c) = db();
    let a = mk_activity(&c, "a");
    let p = oxiline_core::plan::create_plan(
        &c,
        oxiline_core::model::PlanInput {
            date: None, start_minute: 9 * 60, duration_minute: 60,
            weekday_mask: 0b0000001, title: None, activity_ids: vec![a.id.clone()],
        },
    ).unwrap();
    let out = oxiline_core::plan::add_options(&c, &p.id, &[]).unwrap();
    assert!(out.is_empty());
    assert_eq!(sort_orders(&c, &p.id), vec![0]); // unchanged
}

#[test]
fn add_options_missing_plan_is_not_found() {
    let (_f, c) = db();
    let a = mk_activity(&c, "a");
    let err = oxiline_core::plan::add_options(&c, "nope", &[a.id]).unwrap_err();
    assert!(matches!(err, oxiline_core::CoreError::NotFound(_)));
}
```
(`tests/plan.rs` 임포트는 이미 `use rusqlite::Connection;`; `rusqlite::params!`는 fully-qualified 사용.)

- [ ] **Step 4: 코어 빌드 + 테스트**

Run: `cargo test -p oxiline-core --test plan`
Expected: 4 신규 + 기존 plan 테스트 전부 PASS. (기존 `add_option` 참조가 commands.rs에 남아있으면 컴파일 에러 — Task 2에서 정리.)

- [ ] **Step 5: Commit**

```bash
git add crates/oxiline-core/src/plan.rs crates/oxiline-core/tests/plan.rs
git commit -m "feat(core): add_options w/ BEGIN IMMEDIATE; drop single add_option"
```

---

### Task 2: 동시성 스트레스 테스트 (경쟁 판별)

**Files:**
- Test: `crates/oxiline-core/tests/plan.rs`(테스트 1건 추가)

**Interfaces:**
- Consumes: `plan::add_options`(Task 1), `open_and_migrate`, `settings::ensure_defaults`, `mk_activity`.

- [ ] **Step 1: 동시성 테스트 추가**

```rust
#[test]
fn add_options_concurrent_unique_sort_order() {
    use std::sync::Arc;
    use std::thread;
    // Multiple pooled-style connections hammer add_options on the SAME plan in
    // parallel. Under BEGIN IMMEDIATE the write lock is held during the MAX
    // read → every sort_order globally unique, zero errors (busy_timeout waits).
    // A DEFERRED txn would let two connections read the same MAX and insert
    // duplicate sort_orders (or hit SQLITE_BUSY_SNAPSHOT) — fails under DEFERRED.
    let f = NamedTempFile::new().unwrap();
    let setup = oxiline_core::open_and_migrate(f.path()).unwrap();
    oxiline_core::settings::ensure_defaults(&setup).unwrap();
    let a0 = mk_activity(&setup, "seed");
    let p = oxiline_core::plan::create_plan(
        &setup,
        oxiline_core::model::PlanInput {
            date: None, start_minute: 9 * 60, duration_minute: 60,
            weekday_mask: 0b0000001, title: None, activity_ids: vec![a0.id.clone()],
        },
    ).unwrap();

    const THREADS: usize = 4;
    const PER_THREAD: usize = 25;
    let mut buckets: Vec<Vec<String>> = Vec::with_capacity(THREADS);
    for t in 0..THREADS {
        let mut bucket = Vec::with_capacity(PER_THREAD);
        for k in 0..PER_THREAD {
            bucket.push(mk_activity(&setup, &format!("t{t}-k{k}")).id.clone());
        }
        buckets.push(bucket);
    }
    // One connection per thread (mirrors the r2d2 pool; busy_timeout/WAL set per conn).
    let conns: Vec<Connection> = (0..THREADS)
        .map(|_| oxiline_core::open_and_migrate(f.path()).unwrap())
        .collect();
    let plan_id = Arc::new(p.id.clone());

    let handles: Vec<_> = conns.into_iter().zip(buckets.into_iter()).map(|(conn, bucket)| {
        let plan_id = Arc::clone(&plan_id);
        thread::spawn(move || -> Result<(), String> {
            for aid in bucket {
                oxiline_core::plan::add_options(&conn, &plan_id, std::slice::from_ref(&aid))
                    .map_err(|e| e.to_string())?;
            }
            Ok(())
        })
    }).collect();

    for h in handles {
        h.join().unwrap().expect("add_options errored under concurrency");
    }

    let orders = sort_orders(&setup, &p.id);
    assert_eq!(orders.len(), 1 + THREADS * PER_THREAD, "row count mismatch");
    let mut seen = std::collections::HashSet::new();
    for &o in &orders {
        assert!(seen.insert(o), "duplicate sort_order {o}");
    }
}
```

- [ ] **Step 2: 실행 (release로 동시성 의미 보장)**

Run: `cargo test -p oxiline-core --test plan add_options_concurrent --release -- --test-threads=1`
Expected: PASS — zero errors, 101 unique sort_orders. (debug에서도 통과해야 하지만 release로 타이밍 의미 보강.)

- [ ] **Step 3: Commit**

```bash
git add crates/oxiline-core/tests/plan.rs
git commit -m "test(core): concurrent add_options stress (IMMEDIATE race discriminator)"
```

---

### Task 3: app 명령 `add_plan_options` + 등록 + 단일 제거

**Files:**
- Modify: `crates/oxiline-app/src-tauri/src/commands.rs`(L553-561 `add_plan_option` → `add_plan_options`)
- Modify: `crates/oxiline-app/src-tauri/src/lib.rs`(L67 등록 교체)

**Interfaces:**
- Consumes: `plan::add_options`(Task 1), `State<AppState>`, `map_err`, `model::PlanOption`.
- Produces: tauri 명령 `add_plan_options(state, planId: String, activityIds: Vec<String>) -> Result<Vec<PlanOption>, String>`.

- [ ] **Step 1: 명령 교체** (`commands.rs:553-561`)

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

- [ ] **Step 2: 등록 교체** (`lib.rs:67`)

```rust
// before
        commands::add_plan_option,
// after
        commands::add_plan_options,
```

- [ ] **Step 3: 빌드** (specta가 dev에서 `bindings.ts` 재export)

Run: `cargo build -p oxiline-app 2>&1 | tail -20` (또는 `cargo build --manifest-path crates/oxiline-app/src-tauri/Cargo.toml`)
Expected: clean. `bindings.ts`에 `AddPlanOptions` 반영.

- [ ] **Step 4: Commit**

```bash
git add crates/oxiline-app/src-tauri/src/commands.rs crates/oxiline-app/src-tauri/src/lib.rs crates/oxiline-app/src/bindings.ts
git commit -m "feat(app): add_plan_options command; drop single add_plan_option"
```

---

### Task 4: 프론트 api/hook + dnd.tsx 호출부

**Files:**
- Modify: `crates/oxiline-app/src/lib/api.ts`(L167-168 `addPlanOption` → `addPlanOptions`)
- Modify: `crates/oxiline-app/src/hooks.ts`(L306-314 `useAddPlanOption` → `useAddPlanOptions`)
- Modify: `crates/oxiline-app/src/lib/dnd.tsx`(L10 import, L20 hook, L84 호출부)

**Interfaces:**
- Consumes: `invoke`, `PlanOption` 타입, `useQueryClient`.
- Produces: `api.addPlanOptions`, `useAddPlanOptions`, dnd 1회 호출.

- [ ] **Step 1: api.ts 교체** (`api.ts:167-168`)

```ts
  addPlanOptions: (planId: string, activityIds: string[]) =>
    invoke<PlanOption[]>("add_plan_options", { planId, activityIds }),
```

- [ ] **Step 2: hooks.ts 교체** (`hooks.ts:306-314`)

```ts
export function useAddPlanOptions() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (args: { planId: string; activityIds: string[] }) =>
      api.addPlanOptions(args.planId, args.activityIds),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["slots"] });
      qc.invalidateQueries({ queryKey: ["plans"] });
    },
  });
}
```

- [ ] **Step 3: dnd.tsx import + hook + 호출부** (`dnd.tsx:10`, `:20`, `:84`)

```ts
// L10 import
import { useAddPlanOptions, useCreatePlan, useUpdateTask } from "../hooks";
// L20 hook
  const addOptions = useAddPlanOptions();
// L84 호출부
        addOptions.mutate({ planId, activityIds });
```

- [ ] **Step 4: 타입체크**

Run: `cd crates/oxiline-app && npx tsc --noEmit`
Expected: clean (잔여 `addPlanOption`/`useAddPlanOption` 참조 없음).

- [ ] **Step 5: Commit**

```bash
git add crates/oxiline-app/src/lib/api.ts crates/oxiline-app/src/hooks.ts crates/oxiline-app/src/lib/dnd.tsx
git commit -m "feat(app): addPlanOptions/useAddPlanOptions; dnd single bulk call"
```

---

### Task 5: 전체 검증

- [ ] **Step 1: 코어 전체 테스트**

Run: `cargo test -p oxiline-core`
Expected: 전부 PASS (plan 신규 5 + 기존).

- [ ] **Step 2: app 빌드**

Run: `cargo build -p oxiline-app 2>&1 | tail -5`
Expected: clean.

- [ ] **Step 3: 프론트 타입체크 + vitest**

Run: `cd crates/oxiline-app && npx tsc --noEmit && npx vitest run`
Expected: tsc clean, vitest 기존 22/22 유지.

- [ ] **Step 4: 잔여 단일 경로 참조 최종 grep**

Run(grep tool): `add_option\b|addPlanOption\b|useAddPlanOption\b|add_plan_option\b` across `crates/` — 0 matches expected (주석/문서 제외).

- [ ] **Step 5: (정리) followup 문서 업데이트**

`docs/superpowers/2026-08-02-hud-date-orplan-followup.md` 항목 4를 "완료"로 전환(커밋 메시지/검증 요약). 본 태스크는 구현 검증 후 수행.
