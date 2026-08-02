# HUD 보강 · 날짜 popover · OR-계획 다중선택+리사이즈 — 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** recording-네이티브 UI 3종 보강 — HUD를 녹화 세션 기반으로 전면 재구성, 날짜 제목에 월 달력 popover, 활동 다중선택-드래그/드롭-병합/계획 카드 리사이즈.

**Architecture:** 백엔드는 코어 `resize_plan` 신규 + 기존 `add_option`을 tauri 명령으로 노출. 프론트는 각 기능의 순수 로직(now/next·월달력·리사이즈 계산)을 `lib/`에 추출해 vitest(node env, no-DOM)로 TDD하고, 컴포넌트 인터랙션은 `tsc -b && vite build` + 브라우저 smoke로 검증(vitest 설정 참고: `Pointer interactions are verified via the audit/ browser harness, NOT here`).

**Tech Stack:** Rust(oxiline-core rusqlite, Tauri v2 + tauri-specta), React 19 + TypeScript + @dnd-kit/core + @tanstack/react-query + zustand + Tailwind v4. vitest 4(node env).

## Global Constraints

- Tauri 명령 인자는 snake_case(Rust) → camelCase(JS) 자동 변환. `api.ts` 호출은 camelCase 키로.
- 코어 테스트는 `tests/plan.rs` 패턴(`db()` 헬퍼, `2026-08-03` = Monday 가정) 준수.
- 프론트 순수 로직 테스트만 `src/**/*.test.ts` (node env). 컴포넌트/DnD는 vitest 금지.
- `update_plan`은 `PlanInput` 전체를 직접 할당(start/weekday 포함)하므로 리사이즈에 부적합 → 전용 `resize_plan`.
- 커밋 메시지: `feat(core): ...` / `feat(app): ... (Task N)`.
- 기존 색 토큰 재사용: `--color-surface-sunken`, `--color-interactive-primary`, `--popover-radius`, `hueVar()` 등.

## File Structure

- `crates/oxiline-core/src/plan.rs` — `resize_plan` 신규 fn
- `crates/oxiline-core/tests/plan.rs` — resize 테스트
- `crates/oxiline-app/src-tauri/src/commands.rs` — `add_plan_option`, `resize_plan` 명령 + `PlanOption` import
- `crates/oxiline-app/src-tauri/src/lib.rs` — `collect_commands!` 등록 2건
- `crates/oxiline-app/src-tauri/src/hud.rs` — show 이벤트 교체
- `crates/oxiline-app/src/lib/api.ts` — `addPlanOption`, `resizePlan`
- `crates/oxiline-app/src/types.ts` — `PlanOption`
- `crates/oxiline-app/src/hooks.ts` — `useAddPlanOption`, `useResizePlan`
- `crates/oxiline-app/src/lib/now-next.ts` (신규) + 테스트 — 슬롯 now/next 도출
- `crates/oxiline-app/src/lib/calendar.ts` (신규) + 테스트 — 월 달력 그리드
- `crates/oxiline-app/src/lib/resize.ts` (신규) + 테스트 — 리사이즈 계산
- `crates/oxiline-app/src/hud.tsx` — recording-네이티브 재구성
- `crates/oxiline-app/src/components/Header.tsx` — 날짜 popover
- `crates/oxiline-app/src/components/Sidebar.tsx` — 다중선택
- `crates/oxiline-app/src/lib/dnd.tsx` — 다중 페이로드, 드롭-병합, collisionDetection
- `crates/oxiline-app/src/components/RecordTimeline.tsx` — PlanCard droppable + 리사이즈 핸들
- `crates/oxiline-app/src/styles.css` — popover/핸들/선택 스타일

의존: Task1→Task6, Task2→Task5. Task3·Task4는 백엔드 무의존. Task3–6은 서로 독립(병렬 가능).

---

### Task 1: Core `resize_plan`

**Files:**
- Modify: `crates/oxiline-core/src/plan.rs` (insert after `update_plan`, ~line 184)
- Test: `crates/oxiline-core/tests/plan.rs`

**Interfaces:**
- Consumes: `plan::get_plan`, `util::now_iso`, `CoreError`, `params!`, `Connection` (모두 plan.rs에 이미 import 됨).
- Produces: `pub fn resize_plan(conn: &Connection, id: &str, duration_minute: u16) -> Result<Plan>`.

- [ ] **Step 1: Write the failing tests** — append to `crates/oxiline-core/tests/plan.rs`:

```rust
fn mk_activity(c: &Connection, name: &str) -> oxiline_core::model::Activity {
    oxiline_core::activities::create_activity(
        c,
        oxiline_core::model::ActivityInput { name: Some(name.into()), ..Default::default() },
    )
    .unwrap()
}

#[test]
fn resize_plan_updates_duration_only() {
    let (_f, c) = db();
    let a = mk_activity(&c, "코딩");
    let p = oxiline_core::plan::create_plan(
        &c,
        oxiline_core::model::PlanInput {
            date: None,
            start_minute: 9 * 60,
            duration_minute: 60,
            weekday_mask: 0b0000001,
            title: None,
            activity_ids: vec![a.id.clone()],
        },
    )
    .unwrap();
    let r = oxiline_core::plan::resize_plan(&c, &p.id, 120).unwrap();
    assert_eq!(r.duration_minute, 120);
    assert_eq!(r.start_minute, 9 * 60); // unchanged
    assert_eq!(r.weekday_mask, 0b0000001); // unchanged
    // options preserved + slot reflects new duration
    let s = oxiline_core::plan::slots_for_date(&c, "2026-08-03")
        .unwrap()
        .into_iter()
        .find(|s| s.plan_id == p.id)
        .unwrap();
    assert_eq!(s.duration_minute, 120);
    assert_eq!(s.options.len(), 1);
}

#[test]
fn resize_plan_rejects_zero_and_missing() {
    let (_f, c) = db();
    let a = mk_activity(&c, "코딩");
    let p = oxiline_core::plan::create_plan(
        &c,
        oxiline_core::model::PlanInput {
            date: None,
            start_minute: 9 * 60,
            duration_minute: 60,
            weekday_mask: 0b0000001,
            title: None,
            activity_ids: vec![a.id.clone()],
        },
    )
    .unwrap();
    assert!(oxiline_core::plan::resize_plan(&c, &p.id, 0).is_err());
    assert!(oxiline_core::plan::resize_plan(&c, "nope", 30).is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p oxiline-core --test plan resize_plan`
Expected: FAIL — `error[E0425]: cannot find function resize_plan`

- [ ] **Step 3: Implement** — insert into `crates/oxiline-core/src/plan.rs` right after `update_plan` (before `delete_plan`):

```rust
/// Resize a plan's duration in place. Only `duration_minute` changes; start,
/// weekday, title and the OR option set are untouched (unlike `update_plan`,
/// which reassigns start/weekday directly from `PlanInput`). `0` is rejected
/// as a defensive floor — callers clamp to a sensible minimum (e.g. 15 min).
pub fn resize_plan(conn: &Connection, id: &str, duration_minute: u16) -> Result<Plan> {
    if duration_minute == 0 {
        return Err(CoreError::InvalidArgument(
            "duration_minute must be greater than 0".into(),
        ));
    }
    let now = util::now_iso();
    let n = conn.execute(
        "UPDATE plans SET duration_minute = ?1, updated_at = ?2 WHERE id = ?3",
        params![duration_minute as i64, now, id],
    )?;
    if n == 0 {
        return Err(CoreError::NotFound(format!("plan '{id}'")));
    }
    get_plan(conn, id)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p oxiline-core --test plan`
Expected: PASS (기존 2개 + 신규 2개)

- [ ] **Step 5: Commit**

```bash
git add crates/oxiline-core/src/plan.rs crates/oxiline-core/tests/plan.rs
git commit -m "feat(core): resize_plan partial duration update (Task 1)"
```

---

### Task 2: Backend commands + TS plumbing

**Files:**
- Modify: `crates/oxiline-app/src-tauri/src/commands.rs` (imports + 2 commands)
- Modify: `crates/oxiline-app/src-tauri/src/lib.rs` (`collect_commands!`)
- Modify: `crates/oxiline-app/src/lib/api.ts`, `src/types.ts`, `src/hooks.ts`

**Interfaces:**
- Consumes: `plan::add_option(conn, plan_id, activity_id) -> Result<PlanOption>` (코어에 이미 존재, plan.rs:195), `plan::resize_plan` (Task 1).
- Produces: tauri 명령 `add_plan_option(planId, activityId)` / `resize_plan(planId, durationMinute)`; `api.addPlanOption` / `api.resizePlan`; hooks `useAddPlanOption` / `useResizePlan`; 타입 `PlanOption`.

- [ ] **Step 1: Add commands** — in `commands.rs`:

  (a) `PlanOption`을 model import에 추가 (line 9-14 블록):
  ```rust
  use oxiline_core::model::{
      Activity, ActivityInput, CardSuggestion, Category, Compliance, NowContext, Plan, PlanInput,
      PlanOption, PlanSlot, RangeReport, Record, RecordState, RoutineBlock, RoutineStreak, Scope,
      Task, TimelineItem, WeekReport,
  };
  ```

  (b) `delete_plan` 뒤에 두 명령 추가 (line ~550):
  ```rust
  #[tauri::command]
  #[specta::specta]
  pub fn add_plan_option(
      state: State<AppState>,
      plan_id: String,
      activity_id: String,
  ) -> Result<PlanOption, String> {
      plan::add_option(&state.conn(), &plan_id, &activity_id).map_err(map_err)
  }

  #[tauri::command]
  #[specta::specta]
  pub fn resize_plan(
      state: State<AppState>,
      plan_id: String,
      duration_minute: u16,
  ) -> Result<Plan, String> {
      plan::resize_plan(&state.conn(), &plan_id, duration_minute).map_err(map_err)
  }
  ```

- [ ] **Step 2: Register commands** — in `lib.rs` `collect_commands![...]`, 마지막 `commands::delete_plan,` 뒤에:
  ```rust
          commands::delete_plan,
          commands::add_plan_option,
          commands::resize_plan,
      ]);
  ```

- [ ] **Step 3: Build app crate** (specta 바인딩 재생성 + 컴파일 검증)

Run: `cargo build -p oxiline-app`
Expected: BUILD OK (바인딩 파일은 참고용; api.ts는 수동 작성)

- [ ] **Step 4: TS plumbing** —

  (a) `types.ts` — `PlanOption` 인터페이스 추가 (`PlanSlot` 근처):
  ```ts
  /** One alternative within a plan (mirrors `oxiline_core::model::PlanOption`). */
  export interface PlanOption {
    id: string;
    plan_id: string;
    activity_id: string;
    sort_order: number;
  }
  ```

  (b) `api.ts` — `Plan` import에 `PlanOption` 추가 후, `deletePlan` 뒤에:
  ```ts
    deletePlan: (id: string) => invoke<void>("delete_plan", { id }),
    addPlanOption: (planId: string, activityId: string) =>
      invoke<PlanOption>("add_plan_option", { planId, activityId }),
    resizePlan: (planId: string, durationMinute: number) =>
      invoke<Plan>("resize_plan", { planId, durationMinute }),
  ```

  (c) `hooks.ts` — `useCreatePlan` 뒤에:
  ```ts
  export function useAddPlanOption() {
    const qc = useQueryClient();
    return useMutation({
      mutationFn: (args: { planId: string; activityId: string }) =>
        api.addPlanOption(args.planId, args.activityId),
      onSuccess: () => {
        qc.invalidateQueries({ queryKey: ["slots"] });
        qc.invalidateQueries({ queryKey: ["plans"] });
      },
    });
  }

  export function useResizePlan() {
    const qc = useQueryClient();
    return useMutation({
      mutationFn: (args: { planId: string; durationMinute: number }) =>
        api.resizePlan(args.planId, args.durationMinute),
      onSuccess: () => {
        qc.invalidateQueries({ queryKey: ["slots"] });
        qc.invalidateQueries({ queryKey: ["plans"] });
      },
    });
  }
  ```

- [ ] **Step 5: Typecheck + build**

Run: `cd crates/oxiline-app && npx tsc -b && npx vite build`
Expected: PASS (에러 없음)

- [ ] **Step 6: Commit**

```bash
git add crates/oxiline-app/src-tauri/src/commands.rs crates/oxiline-app/src-tauri/src/lib.rs \
        crates/oxiline-app/src/lib/api.ts crates/oxiline-app/src/types.ts crates/oxiline-app/src/hooks.ts
git commit -m "feat(app): add_plan_option + resize_plan commands & TS plumbing (Task 2)"
```

---

### Task 3: HUD recording-네이티브 재구성

**Files:**
- Create: `crates/oxiline-app/src/lib/now-next.ts`
- Test: `crates/oxiline-app/src/lib/__tests__/now-next.test.ts`
- Modify: `crates/oxiline-app/src/hud.tsx`
- Modify: `crates/oxiline-app/src-tauri/src/hud.rs` (show 이벤트)

**Interfaces:**
- Consumes: `PlanSlot` 타입, hooks `useRecordState`/`useSlots`/`useCompliance`, `record-format`(`hmm`/`complianceLabel`/`hueVar`).
- Produces: `currentSlot(slots, nowMin)` / `nextSlot(slots, nowMin)`; 재구성된 `HudCard`.

- [ ] **Step 1: Write failing test** — `crates/oxiline-app/src/lib/__tests__/now-next.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { currentSlot, nextSlot } from "../now-next";
import type { PlanSlot } from "../../types";

function slot(id: string, start: number, dur: number): PlanSlot {
  return { plan_id: id, date: "2026-08-02", start_minute: start, duration_minute: dur, options: [], is_resolved: false, resolved_by: null };
}

describe("now/next derivation", () => {
  const slots = [slot("a", 600, 60), slot("b", 720, 30), slot("c", 900, 45)]; // 10:00-11:00, 12:00-12:30, 15:00-15:45
  it("current = slot containing now (inclusive start, exclusive end)", () => {
    expect(currentSlot(slots, 600)?.plan_id).toBe("a");
    expect(currentSlot(slots, 659)?.plan_id).toBe("a");
    expect(currentSlot(slots, 660)?.plan_id).toBeNull();
  });
  it("next = first slot strictly after now", () => {
    expect(nextSlot(slots, 600)?.plan_id).toBe("b");
    expect(nextSlot(slots, 659)?.plan_id).toBe("b");
    expect(nextSlot(slots, 945)?.plan_id).toBeNull();
  });
  it("empty slots → null", () => {
    expect(currentSlot([], 0)).toBeNull();
    expect(nextSlot([], 0)).toBeNull();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd crates/oxiline-app && npx vitest run src/lib/__tests__/now-next.test.ts`
Expected: FAIL — cannot find `../now-next`

- [ ] **Step 3: Implement helper** — `crates/oxiline-app/src/lib/now-next.ts`:

```ts
import type { PlanSlot } from "../types";

/** The slot whose [start, start+duration) contains `nowMin`, else null. */
export function currentSlot(slots: PlanSlot[], nowMin: number): PlanSlot | null {
  return (
    slots.find(
      (s) => nowMin >= s.start_minute && nowMin < s.start_minute + s.duration_minute,
    ) ?? null
  );
}

/** First slot starting strictly after `nowMin` (earliest), else null. */
export function nextSlot(slots: PlanSlot[], nowMin: number): PlanSlot | null {
  return (
    [...slots]
      .filter((s) => s.start_minute > nowMin)
      .sort((a, b) => a.start_minute - b.start_minute)[0] ?? null
  );
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd crates/oxiline-app && npx vitest run src/lib/__tests__/now-next.test.ts`
Expected: PASS (3)

- [ ] **Step 5: Rewrite `hud.tsx` HudCard** — `onNowUpdate`/`NowContext` 제거, recording-네이티브로 교체. 핵심 변경:

  - imports: `useRecordState`, `useSlots`, `useCompliance` (`../hooks`), `todayStr` 유지, `onNowUpdate` 제거, `useQueryClient` 추가, `listen` import 유지(show 이벤트용), `currentSlot`/`nextSlot`/`PlanSlot`/`ActiveSession` import, `record-format`의 `hmm`/`hueVar` import.
  - 상태: `const stateQ = useRecordState(); const slotsQ = useSlots(todayStr()); const weekQ = useCompliance("week");` (기존 `ctx`/`catsQ`/`tlQ` 중 `tlQ`는 OxideBar용 유지).
  - show 리프레시: `useEffect(() => { const un = listen("oxiline://hud-show", () => { qc.invalidateQueries(); }); return () => { void un.then((f) => f()); }; }, []);`
  - `nowMin = new Date().getHours()*60 + new Date().getMinutes()`; `const cur = currentSlot(slotsQ.data ?? [], nowMin)`; `const nxt = nextSlot(slotsQ.data ?? [], nowMin)`.
  - 본문 렌더:
    - **녹화 중** (`stateQ.data?.active`): `● {active.activity.name}` + `hmm(active.elapsed_seconds) 경과` + 주간 막대(`weekQ.data`에서 `activity.id`로 Compliance 찾기 → `hmm(recorded)/hmm(target)` + 폭 `Math.min(100, ratio*100)%` + `hueVar(activity.hue_label)`).
    - **자유 시간**: `cur ? 지금 예정 · {cur.options[0]?.name ?? "계획"}${cur.options.length>1?" OR":""}` : `지금 · 자유 시간`.
    - **다음** (`nxt`): `다음 · {first opt}${nxt.options.length>1?" OR":""} · {hhmm(start)} ({start-nowMin}분 후)`. 없으면 생략.
  - OxideBar 행은 유지(`items={tlQ.data ?? []} …`).

- [ ] **Step 6: Swap HUD show event** — `crates/oxiline-app/src-tauri/src/hud.rs` `show()`에서 레거시 ctx 계산/emit 블록을 단순 이벤트로 교체:

  ```rust
  pub fn show(app: &AppHandle) {
      let Some(hud) = app.get_webview_window("hud") else { return; };
      let _ = hud.emit("oxiline://hud-show", ());
      position_top_center(&hud);
      let _ = hud.show();
      let state = app.state::<AppState>();
      let duration = oxiline_core::settings::get_i64(&state.conn(), "hud_duration_ms", 2000).max(500) as u64;
      let now = Instant::now();
      *LATEST_SHOW.lock() = Some(now);
      let app = app.clone();
      std::thread::spawn(move || { /* 기존 auto-hide 로직 유지 — 재읽기 후 동일하게 */ });
  }
  ```
  (auto-hide 스레드 본문은 기존 그대로 복사 — `hud.rs:45-55` 재확인 후 동일 유지. `timeline::get_now_context` import 사용 제거.)

- [ ] **Step 7: Typecheck + build + smoke**

Run: `cd crates/oxiline-app && npx tsc -b && npx vite build`
Expected: PASS
Smoke: `pnpm -C crates/oxiline-app dev` (또는 audit mock) → HUD ⌘⇧O: 녹화 중이면 `● 활동·경과·주간 막대`, 아니면 `자유 시간/지금 예정` + `다음 …`.

- [ ] **Step 8: Commit**

```bash
git add crates/oxiline-app/src/lib/now-next.ts crates/oxiline-app/src/lib/__tests__/now-next.test.ts \
        crates/oxiline-app/src/hud.tsx crates/oxiline-app/src-tauri/src/hud.rs
git commit -m "feat(app): recording-native HUD rework (Task 3)"
```

---

### Task 4: 날짜 popover — 월 달력 + 기록 마커

**Files:**
- Create: `crates/oxiline-app/src/lib/calendar.ts`
- Test: `crates/oxiline-app/src/lib/__tests__/calendar.test.ts`
- Modify: `crates/oxiline-app/src/components/Header.tsx`, `src/styles.css`

**Interfaces:**
- Consumes: `useTimelineRange(from, to)` (각 날짜 `items`/`category_id`/`is_skipped`), `useUi`(`date`,`setDate`,`setView`), `categoryById`/`categoryColor`.
- Produces: `monthGrid(date)` / `monthBounds(date)`; Header 날짜 popover.

- [ ] **Step 1: Write failing test** — `crates/oxiline-app/src/lib/__tests__/calendar.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { monthGrid, monthBounds } from "../calendar";

describe("monthGrid (Mon-first)", () => {
  it("2026-08 starts Mon 2026-07-27 and spans 42 cells", () => {
    const g = monthGrid("2026-08-15");
    expect(g).toHaveLength(42);
    expect(g[0]).toBe("2026-07-27"); // 2026-08-01 is a Saturday → Mon offset 5
    expect(g).toContain("2026-08-01");
    expect(g).toContain("2026-08-31");
  });
  it("bounds = first..last cell", () => {
    const b = monthBounds("2026-08-15");
    expect(b.from).toBe("2026-07-27");
    expect(b.to).toBe("2026-09-05"); // 42 cells from 07-27
  });
  it("handles December→January wrap", () => {
    const g = monthGrid("2026-12-10");
    expect(g).toContain("2026-12-01");
    expect(g).toContain("2027-01-31");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd crates/oxiline-app && npx vitest run src/lib/__tests__/calendar.test.ts`
Expected: FAIL — cannot find `../calendar`

- [ ] **Step 3: Implement helper** — `crates/oxiline-app/src/lib/calendar.ts`:

```ts
function pad(n: number): string {
  return String(n).padStart(2, "0");
}
function ymd(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}
function parse(s: string): Date {
  const [y, m, d] = s.split("-").map(Number);
  return new Date(y, m - 1, d);
}

/** 6×7 (42-cell) Mon-first grid of YYYY-MM-DD strings for the month of `date`. */
export function monthGrid(date: string): string[] {
  const first = parse(date);
  first.setDate(1);
  const jsDow = first.getDay(); // Sun=0..Sat=6
  const monOffset = jsDow === 0 ? 6 : jsDow - 1; // Mon=0..Sun=6
  const start = new Date(first);
  start.setDate(first.getDate() - monOffset);
  const cells: string[] = [];
  for (let i = 0; i < 42; i++) {
    const d = new Date(start);
    d.setDate(start.getDate() + i);
    cells.push(ymd(d));
  }
  return cells;
}

/** `[firstCell, lastCell]` range for `useTimelineRange` (covers adjacent-month spillover). */
export function monthBounds(date: string): { from: string; to: string } {
  const g = monthGrid(date);
  return { from: g[0], to: g[g.length - 1] };
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd crates/oxiline-app && npx vitest run src/lib/__tests__/calendar.test.ts`
Expected: PASS (3)

- [ ] **Step 5: Wire popover into Header.tsx** —

  (a) imports 추가: `useState`, `useRef`, `useEffect`; `monthGrid`, `monthBounds` (`../lib/calendar`); `todayStr` 이미 있음.
  (b) popover 상태: `const [calOpen, setCalOpen] = useState(false);` + `const [calMonth, setCalMonth] = useState(date);` (표시 중인 월). ref/외부클릭:
  ```ts
  const popRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!calOpen) return;
    function onDown(e: PointerEvent) {
      if (popRef.current && !popRef.current.contains(e.target as Node)) setCalOpen(false);
    }
    window.addEventListener("pointerdown", onDown);
    return () => window.removeEventListener("pointerdown", onDown);
  }, [calOpen]);
  ```
  (c) 마커 데이터: `const bounds = monthBounds(calMonth); const monthQ = useTimelineRange(bounds.from, bounds.to); const byDate = new Map((monthQ.data ?? []).map(c => [c.date, c.items] as const));`
  (d) 날짜 제목 버튼: `onClick`을 `goToToday`에서 `() => setCalOpen(v => !v)`로 변경(토글). 옆에 작은 chevron/캘럿으로 열림 표시.
  (e) popover 패널(`popRef` 부착, `calOpen`일 때만 렌더, `role="dialog"`): 헤더 `‹ {yy}년 {mm}월 › [오늘]`; 본문 = 월~일 헤더 + `monthGrid(calMonth).map(cell => …)` 각 셀:
    - 다른 달(`cell.slice(0,7) !== calMonth.slice(0,7)`) = `text-text-subtle/40`.
    - 오늘(`cell === today`) = 채운 원 강조; 선택일(`cell === date`) = 링.
    - 마커: `byDate.get(cell)`에서 `is_skipped` 제외 → hue 집계 → 최대 5개 색 점(주간 스트립 `Header.tsx:107` 기존 로직과 동일: `categoryById(categories, i.category_id)?.color_hue`).
    - 클릭: `setDate(cell); setView("today"); setCalOpen(false);`.
  - `‹`/`›`: `setCalMonth(shift(calMonth, ±35))` (월 단위 이동; 35일 shift로 대략 한 달). `[오늘]`: `setCalMonth(today); setDate(today); setView("today"); setCalOpen(false);`.

- [ ] **Step 6: styles** — `styles.css`에 popover 클래스 추가 (기존 토큰 재사용):
  ```css
  .date-popover { position: absolute; left: 0; top: 100%; z-index: 50; margin-top: 4px;
    border-radius: var(--popover-radius); border: 1px solid var(--color-border);
    background: var(--color-surface-raised); box-shadow: var(--shadow-lg); padding: 10px; }
  .date-popover-grid { display: grid; grid-template-columns: repeat(7, 1fr); gap: 2px; }
  .date-popover-cell { aspect-ratio: 1; display: flex; flex-direction: column; align-items: center;
    justify-content: center; border-radius: var(--radius-full); font-size: 12px; cursor: pointer; }
  ```

- [ ] **Step 7: Typecheck + build + smoke**

Run: `cd crates/oxiline-app && npx tsc -b && npx vite build`
Expected: PASS
Smoke: 날짜 제목 클릭 → 이번 달 그리드 + 오늘 강조 + 색 점; ‹ › 월 이동; 날짜 클릭 → 해당일 이동 + 닫힘; 외부 클릭 → 닫힘.

- [ ] **Step 8: Commit**

```bash
git add crates/oxiline-app/src/lib/calendar.ts crates/oxiline-app/src/lib/__tests__/calendar.test.ts \
        crates/oxiline-app/src/components/Header.tsx crates/oxiline-app/src/styles.css
git commit -m "feat(app): date popover month calendar with record markers (Task 4)"
```

---

### Task 5: OR-계획 다중선택 + 드롭-병합

**Files:**
- Modify: `crates/oxiline-app/src/components/Sidebar.tsx`, `src/lib/dnd.tsx`, `src/components/RecordTimeline.tsx`, `src/styles.css`

**Interfaces:**
- Consumes: `useCreatePlan` (기존), `useAddPlanOption` (Task 2), `useDroppable`, `rectIntersection` (@dnd-kit/core).
- Produces: 다중선택 드래그 → OR 계획; 기존 카드 위 드롭 → 옵션 추가.

- [ ] **Step 1: Sidebar 다중선택** — `Sidebar.tsx` `ActivityLibrary`:
  - `const [selected, setSelected] = useState<Set<string>>(new Set());`
  - 각 `DraggableActivity`에 `selected: boolean`, `onSelect: (id:string, additive:boolean) => void` prop 전달.
  - `onSelect`: `additive`(metaKey/ctrlKey)면 토글 추가; 아니면 `{id}` 단일 선택. 빈 곳 클릭 해제는 목록 컨테이너 `onPointerDown`에서(빈 영역).
  - `DraggableActivity`: 카드 `onClick={() => onSelect(activity.id, e.metaKey||e.ctrlKey)}` 추가. 드래그 페이로드 변경:
    ```ts
    const ids = selected.size > 0 && selected.has(activity.id) ? [...selected] : [activity.id];
    useDraggable({ id: `activity-${activity.id}`, data: { kind: "activity", activityIds: ids } });
    ```
    (기존 `activityId` 단일 → `activityIds` 배열로 교체.)
  - 시각: `selected` 카드에 `ring-2 ring-interactive-primary`; 드래그 중(`isDragging`)이면 선택 카운트 배지 `{ids.length}` 옵션 표시.

- [ ] **Step 2: dnd.tsx 페이로드 + 드롭-병합 + collisionDetection** —
  - import: `rectIntersection` (`@dnd-kit/core`), `useAddPlanOption` (`../hooks`).
  - `const addOption = useAddPlanOption();`
  - `handleDragEnd` activity 분기 교체:
    ```ts
    } else if (data.kind === "activity") {
      const activityIds = (data as { activityIds: string[] }).activityIds;
      if (overData.kind === "plan-slot") {
        const planId = (overData as { planId: string }).planId;
        activityIds.forEach((aid) => addOption.mutate({ planId, activityId: aid }));
      } else {
        createPlan.mutate({
          date: overData.date as string,
          start_minute: dropMinute,
          duration_minute: 60,
          weekday_mask: 0,
          title: null,
          activity_ids: activityIds,
        });
      }
    }
    ```
  - `<DndContext sensors={[pointerSensor]} collisionDetection={rectIntersection} onDragEnd={handleDragEnd}>`.

- [ ] **Step 3: RecordTimeline PlanCard를 droppable로** — `RecordTimeline.tsx`:
  - `PlanLane`의 인라인 카드를 별도 컴포넌트 `PlanCard`로 추출(훅 규칙: 슬롯마다 `useDroppable`).
    ```tsx
    function PlanCard({ s, dayStartMin }: { s: PlanSlot; dayStartMin: number }) {
      const { setNodeRef } = useDroppable({
        id: `plan-${s.plan_id}`,
        data: { kind: "plan-slot", planId: s.plan_id },
      });
      const top = (s.start_minute - dayStartMin) * PX_PER_MIN;
      const height = s.duration_minute * PX_PER_MIN;
      return (
        <div ref={setNodeRef} className="absolute left-1 right-1 overflow-hidden rounded-md border border-dashed border-border-strong p-1.5" style={{ top, height }}>
          {/* 기존 OR/옵션 렌더 그대로 */}
        </div>
      );
    }
    ```
  - `PlanLane`: `{slots.map(s => <PlanCard key={s.plan_id} s={s} dayStartMin={dayStartMin} />)}`.

- [ ] **Step 4: styles** — 선택 링 클래스(필요시), 병합-가능 hover(카드 droppable over 시 `ring` 강조 — `useDroppable`의 `isOver`로 `PlanCard`에 `isOver && "ring-2 ring-interactive-primary"` 추가 가능).

- [ ] **Step 5: Typecheck + build + smoke**

Run: `cd crates/oxiline-app && npx tsc -b && npx vite build`
Expected: PASS
Smoke: 활동 2개 ⌘클릭 선택 → 타임라인 드롭 → OR 계획(옵션 2); 활동 1개 → 기존 카드 위 드롭 → 옵션 추가(색 점/옵션 증가).

- [ ] **Step 6: Commit**

```bash
git add crates/oxiline-app/src/components/Sidebar.tsx crates/oxiline-app/src/lib/dnd.tsx \
        crates/oxiline-app/src/components/RecordTimeline.tsx crates/oxiline-app/src/styles.css
git commit -m "feat(app): OR-plan multi-select drag + drop-to-merge (Task 5)"
```

---

### Task 6: 계획 카드 리사이즈 핸들

**Files:**
- Create: `crates/oxiline-app/src/lib/resize.ts`
- Test: `crates/oxiline-app/src/lib/__tests__/resize.test.ts`
- Modify: `crates/oxiline-app/src/components/RecordTimeline.tsx` (PlanCard 핸들), `src/styles.css`

**Interfaces:**
- Consumes: `SNAP_MINUTES` (`./dnd`), `PX_PER_MIN` (RecordTimeline 로컬), `useResizePlan` (Task 2).
- Produces: `resizeDuration(currentMin, deltaMin, min)`; PlanCard 하단 리사이즈 핸들.

- [ ] **Step 1: Write failing test** — `crates/oxiline-app/src/lib/__tests__/resize.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { resizeDuration } from "../resize";

describe("resizeDuration", () => {
  it("snaps to 5 min", () => {
    expect(resizeDuration(60, 8)).toBe(65); // 68 → snap 70? see impl: round(68/5)*5=70
    expect(resizeDuration(60, 7)).toBe(70);
  });
  it("clamps to minimum", () => {
    expect(resizeDuration(60, -55, 15)).toBe(15); // 5 → clamp 15
    expect(resizeDuration(60, -200, 15)).toBe(15);
  });
  it("shrinks by delta", () => {
    expect(resizeDuration(120, -30, 15)).toBe(90);
  });
});
```
> Note: snap = `round((current+delta)/5)*5`. `60+8=68→70`, `60+7=67→65`? `round(67/5)=round(13.4)=13→65`. 위 케이스는 구현에 맞춰 조정 — 구현 후 `68→70`, `67→65` 임을 확인하고 테스트 값 확정.

- [ ] **Step 2: Run test to verify it fails**

Run: `cd crates/oxiline-app && npx vitest run src/lib/__tests__/resize.test.ts`
Expected: FAIL — cannot find `../resize`

- [ ] **Step 3: Implement helper** — `crates/oxiline-app/src/lib/resize.ts`:

```ts
import { SNAP_MINUTES } from "./dnd";

/** New duration after dragging a resize handle by `deltaMin` minutes.
 *  Snaps to SNAP_MINUTES (5) and clamps to `min` (default 15). */
export function resizeDuration(currentMin: number, deltaMin: number, min = 15): number {
  const snapped = Math.round((currentMin + deltaMin) / SNAP_MINUTES) * SNAP_MINUTES;
  return Math.max(min, snapped);
}
```

- [ ] **Step 4: Run test to verify it passes — fix expected values from Step 1 if the snap math differs**

Run: `cd crates/oxiline-app && npx vitest run src/lib/__tests__/resize.test.ts`
Expected: PASS (테스트 기대값을 구현에 맞춰 확정: `resizeDuration(60,8)=70`, `resizeDuration(60,7)=65`)

- [ ] **Step 5: Add resize handle to PlanCard** — `RecordTimeline.tsx` `PlanCard`:
  - imports: `useState`, `useRef`; `resizeDuration` (`../lib/resize`); `useResizePlan` (`../hooks`).
  - 상태: `const [dragDur, setDragDur] = useState<number | null>(null);` `const resize = useResizePlan();`
  - 높이: `const height = (dragDur ?? s.duration_minute) * PX_PER_MIN;`
  - 핸들 요소(카드 하단):
    ```tsx
    <div
      className="absolute inset-x-0 bottom-0 h-1.5 cursor-ns-resize opacity-0 transition group-hover:opacity-100"
      onPointerDown={(e) => {
        e.stopPropagation();
        const startY = e.clientY;
        const startDur = s.duration_minute;
        (e.currentTarget as Element).setPointerCapture(e.pointerId);
        const move = (ev: PointerEvent) => setDragDur(resizeDuration(startDur, (ev.clientY - startY) / PX_PER_MIN));
        const up = (ev: PointerEvent) => {
          window.removeEventListener("pointermove", move);
          window.removeEventListener("pointerup", up);
          setDragDur((h) => { if (h != null) resize.mutate({ planId: s.plan_id, durationMinute: h }); return null; });
        };
        window.addEventListener("pointermove", move);
        window.addEventListener("pointerup", up);
      }}
    />
    ```
  - 카드 컨테이너에 `group` 클래스 추가(hover 핸들 노출용).

- [ ] **Step 6: styles** — `styles.css`: 핸들 `.cursor-ns-resize`는 Tailwind 클래스로 충분; 추가 불필요(위 className 사용). hover 노출은 `group-hover:opacity-100`로 처리.

- [ ] **Step 7: Typecheck + build + smoke**

Run: `cd crates/oxiline-app && npx tsc -b && npx vite build`
Expected: PASS
Smoke: 카드 hover → 하단 핸들 노출; 드래그 → 5분 스냅으로 높이 변화; 놓으면 duration 갱신(새로고침 후 유지); 핸들 드래그 중 타임라인 드롭존 반응 없음(stopPropagation).

- [ ] **Step 8: Commit**

```bash
git add crates/oxiline-app/src/lib/resize.ts crates/oxiline-app/src/lib/__tests__/resize.test.ts \
        crates/oxiline-app/src/components/RecordTimeline.tsx
git commit -m "feat(app): plan card resize handle (Task 6)"
```

---

## Self-Review (작성자 점검)

**1. Spec coverage:**
- A. HUD 녹화-네이티브 → Task 3 (now/next 헬퍼 + hud.tsx + hud.rs 이벤트). ✓
- B. 날짜 popover 월달력+마커 → Task 4 (calendar 헬퍼 + Header popover). ✓
- C1 다중선택-드래그 → Task 5 Step 1–2. ✓
- C2 드롭-병합 → Task 5 Step 2–3 (addPlanOption + plan-card droppable + rectIntersection). ✓
- C3 리사이즈 → Task 6 (resize 헬퍼 + 핸들). ✓
- 백엔드 `resize_plan` → Task 1; `add_plan_option` → Task 2. ✓

**2. Placeholder scan:** "TODO/TBD" 없음. Step 1 테스트 값에 snap 수학 확인 노트 포함(Step 4에서 확정) — 자리표시자 아님, 검증 게이트.

**3. Type consistency:**
- `activityIds: string[]` — Sidebar(dnd payload) ↔ dnd.tsx(cast) ↔ createPlan(`activity_ids`). ✓
- `useAddPlanOption({planId, activityId})` — hooks ↔ dnd.tsx. ✓
- `useResizePlan({planId, durationMinute})` — hooks ↔ RecordTimeline. ✓
- `PlanOption` — model.rs ↔ types.ts(id/plan_id/activity_id/sort_order). ✓
- `resize_plan(conn, id, duration_minute: u16)` — core ↔ command(`duration_minute: u16`) ↔ api(`durationMinute: number`). ✓ (u16 ↔ number; 프론트에서 0–65535 보장)

## 의존 그래프

```
Task1 (core resize_plan) ──┐
                           ├──▶ Task6 (resize handle)
Task2 (commands + plumbing)─┤
                           ├──▶ Task5 (drop-merge uses addPlanOption)
                           └──▶ (all UI tasks use tsc/build)
Task3 (HUD)      ── 독립
Task4 (popover)  ── 독립
```
Task1·Task2 먼저(순차 OK, 서로 독립이므로 병렬 가능). 이후 Task3·4·5·6 병렬 가능(Task5는 Task2 완료 후, Task6은 Task1 완료 후).
