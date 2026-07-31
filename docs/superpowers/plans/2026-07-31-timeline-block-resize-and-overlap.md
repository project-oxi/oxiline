# 타임라인 블록 리사이즈 + 드래그 생성 + 겹침 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 타임라인 카드를 드래그해 길이를 자유롭게 늘리고, 한 시간대에 여러 카드를 겹쳐 배치하며, 빈 시간대를 드래그해 원하는 길이의 블록을 한 번에 만든다.

**Architecture:** 프론트엔드 전용. Rust/스키마 변경 없음 — `create_task`/`update_task`가 이미 `startMinute`/`durationMinute`을 받는다. 이동 드래그는 기존 dnd-kit 그대로 두고, 리사이즈 핸들과 드래그-생성은 격리된 pointer-event 핸들러(`stopPropagation`)로 추가. 겹침의 3열 소프트캡은 `layout()`(순수 유지) 밖, `DayTimeline` 렌더 레이어에서 적용.

**Tech Stack:** React 19, TypeScript 5.7, @dnd-kit/core 6.3, TanStack Query 5, Zustand, Tailwind v4, Vite 6, Tauri 2. 패키지 매니저: **bun** (`bun.lock`). 새 devDep: `vitest`.

## Global Constraints

- **백엔드 변경 없음.** 마이그레이션·Rust 도메인 타입 변경 금지. 모든 작업은 `crates/oxiline-app/src/**` 안.
- **타입체크 게이트:** 모든 작업 종료 후 `cd crates/oxiline-app && bun run build` (= `tsc -b && vite build`) 가 에러/경고 없이 통과해야 한다. `vite.config.ts`의 `rollupOptions.input`은 `{main, hud}`만이므로 `audit/`·`*.test.ts`는 빌드에 포함되지 않는다.
- **검증 모드(작업별 명시):**
  - **[UNIT]** 순수 로직 → `bun run test` (vitest, node 환경). 대상: Task 2.
  - **[BROWSER]** 포인터 상호작용 → `tauri-v2-browser-audit-mock` 스킬로 `audit/` 하네스를 통해 실제 컴포넌트를 헤드리스 브라우저에서 구동 (`page.mouse` 드래그 + `window.__mockLog` 단언). 대상: Task 4, 5, 6.
- **스냅 단위:** 이동=5분(`dnd.tsx` `SNAP_MINUTES`), 리사이즈·생성=15분.
- **복사 규칙(§7.11):** 빈 상태는 "왜+다음 행동", 에러는 사과 없이 사실+다음 행동, 버튼 동사는 결과와 일치.
- **i18n:** 사용자 가시 문자열은 모두 `t()` 경유, `locales/ko.json`·`locales/en.json` 양쪽에 키 추가.
- **TDD 원칙:** [UNIT] 작업은 "테스트 먼저 → 실패 확인 → 구현 → 통과 확인" 순서를 지킨다. [BROWSER] 작업은 감사 모크가 곧 테스트 사이클이다(가짜 RTL/jsdom 부트스트랩 금지 — 사용자가 요청하지 않은 스코프).

---

## File Structure

| 파일 | 책임 | 작업 |
|---|---|---|
| `vitest.config.ts` (신규) | vitest 설정 (node env, `src/**/*.test.ts`) | T1 |
| `package.json` (수정) | `vitest` devDep, `test`/`test:watch` 스크립트 | T1 |
| `src/lib/timeline-math.ts` (신규) | 순수: snap/clamp/cluster/partition | T2 |
| `src/lib/__tests__/timeline-math.test.ts` (신규) | T2 단위 테스트 | T2 |
| `audit/mock.ts` (신규, 스캐폴드) | `__TAURI_INTERNALS__` 모킹 + 시드 + `__mockLog` | T3 |
| `audit/main.tsx` (신규, 스캐폴드) | mock 먼저 임포트 후 `src/main` 부트 | T3 |
| `audit/index.html` (신규, 스캐폴드) | audit 엔트리 HTML | T3 |
| `src/components/BlockView.tsx` (수정) | 하단 리사이즈 그립 + `Shift+↑/↓` | T4 |
| `src/components/DayTimeline.tsx` (수정) | 드래그-생성 + `didDrag` 억제 + 고무줄 + 소프트캡 렌더 + "+N 더 보기" | T5, T6 |
| `locales/ko.json`, `locales/en.json` (수정) | 신규 문자열 키 | T7 |

`audit/`는 검증용 스캐폴드 — **T7에서 제거** (프로덕션 빌드 입력은 `{main, hud}` 그대로 유지).

---

## Task 1: Vitest 검증 하네스  **[UNIT-setup]**

순수 로직을 진짜 TDD 하기 위한 최소 테스트 러너. 포인터 상호작용에는 쓰지 않는다.

**Files:**
- Modify: `crates/oxiline-app/package.json`
- Create: `crates/oxiline-app/vitest.config.ts`
- Create: `crates/oxiline-app/src/lib/__tests__/sanity.test.ts`

**Interfaces:**
- Produces: `bun run test` 명령(모든 [UNIT] 검증의 진입점).

- [ ] **Step 1: vitest 설치 + 스크립트 추가**

Run:
```bash
cd crates/oxiline-app && bun add -d vitest
```

`package.json`의 `scripts`와 `devDependencies`에 자동 반영된다. 수동으로 `scripts` 블록이 아래와 같게 확인/수정:
```json
"scripts": {
  "dev": "vite",
  "build": "tsc -b && vite build",
  "preview": "vite preview",
  "test": "vitest run",
  "test:watch": "vitest"
},
```

- [ ] **Step 2: vitest 설정 작성**

`vitest.config.ts`:
```ts
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Pure-logic tests only (node env — no DOM/jsdom). Pointer interactions are
// verified via the audit/ browser harness, NOT here.
export default defineConfig({
  plugins: [react()],
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
```

- [ ] **Step 3: sanity 테스트 작성 (runner 동작 확인)**

`src/lib/__tests__/sanity.test.ts`:
```ts
import { describe, it, expect } from "vitest";

describe("sanity", () => {
  it("runs", () => {
    expect(1 + 1).toBe(2);
  });
});
```

- [ ] **Step 4: 실행 → PASS 확인**

Run: `cd crates/oxiline-app && bun run test`
Expected: 1 test passed.

- [ ] **Step 5: 커밋**

```bash
git add crates/oxiline-app/package.json crates/oxiline-app/bun.lock crates/oxiline-app/vitest.config.ts crates/oxiline-app/src/lib/__tests__/sanity.test.ts
git commit -m "test(app): add vitest harness for pure-logic tests"
```

---

## Task 2: 순수 타임라인 수학 (snap / clamp / cluster / partition)  **[UNIT]**

리사이즈·생성 스냅/클램프와 겹침 소프트캡 선택 규칙을 순수 함수로 뽑아 TDD. spec §4 선택 규칙(시작 오름차순, 길이 내림차순, id)은 미묘하게 틀리기 쉬워 자동 테스트가 필수.

**Files:**
- Create: `crates/oxiline-app/src/lib/timeline-math.ts`
- Test: `crates/oxiline-app/src/lib/__tests__/timeline-math.test.ts`

**Interfaces:**
- Produces:
  - `snapMinute(m: number, step: number): number` — 반올림 + `[0, 1440-step]` 클램프.
  - `clampDuration(start, dur, dayEndMin, minDur?): number` — `[minDur, dayEndMin - start]`.
  - `groupClusters(items: TimelineItem[]): TimelineItem[][]` — 최대 중첩 클러스터.
  - `partitionCluster(cluster, cap): { visible: TimelineItem[]; overflow: TimelineItem[] }` — 선택 규칙 적용.

- [ ] **Step 1: 실패 테스트 작성**

`src/lib/__tests__/timeline-math.test.ts`:
```ts
import { describe, it, expect } from "vitest";
import { snapMinute, clampDuration, groupClusters, partitionCluster } from "../timeline-math";
import type { TimelineItem } from "../../types";

const item = (id: string, start: number, dur: number): TimelineItem => ({
  id, is_virtual: false, title: id, start_minute: start, duration_minute: dur,
  category_id: null, is_done: false, is_skipped: false, origin_routine_block_id: null,
});

describe("snapMinute", () => {
  it("rounds to step", () => {
    expect(snapMinute(17, 15)).toBe(15);
    expect(snapMinute(23, 15)).toBe(30);
  });
  it("clamps to [0, 1440-step]", () => {
    expect(snapMinute(-5, 15)).toBe(0);
    expect(snapMinute(2000, 15)).toBe(1425);
  });
});

describe("clampDuration", () => {
  it("floors at minDur and ceilings at dayEndMin - start", () => {
    expect(clampDuration(600, 5, 1320)).toBe(15);       // floor
    expect(clampDuration(600, 9999, 1320)).toBe(720);   // 1320-600
    expect(clampDuration(600, 45, 1320)).toBe(45);      // passthrough
  });
  it("never goes below minDur even if window smaller", () => {
    // window to dayEnd (=10) < minDur (=15): ceiling wins — never extend past dayEnd (spec §6)
    expect(clampDuration(1310, 30, 1320, 15)).toBe(10);
  });
});

describe("groupClusters", () => {
  it("groups overlapping, splits disjoint", () => {
    const a = item("a", 540, 60);   // 9:00-10:00
    const b = item("b", 555, 60);   // 9:15-10:15  (overlaps a)
    const c = item("c", 660, 30);   // 11:00-11:30 (disjoint)
    const clusters = groupClusters([c, a, b]); // unsorted input
    expect(clusters).toHaveLength(2);
    expect(clusters[0].map((i) => i.id)).toEqual(["a", "b"]);
    expect(clusters[1].map((i) => i.id)).toEqual(["c"]);
  });
  it("item-count semantics: staggered 4-item cluster (max 3 concurrent) is still ONE cluster", () => {
    // a 9:00-10:00, b 9:15-10:15, c 9:45-11:00, d 10:30-11:30
    // never 4 concurrent (max depth 3), but all linked into one cluster → item-count triggers chip
    const a = item("a", 540, 60);
    const b = item("b", 555, 60);
    const c = item("c", 585, 75);
    const d = item("d", 630, 60);
    const clusters = groupClusters([a, b, c, d]);
    expect(clusters).toHaveLength(1);
    expect(clusters[0]).toHaveLength(4);
    // cap=3 on this 4-item cluster → 1 overflow despite max-concurrent being only 3
    expect(partitionCluster(clusters[0], 3).overflow).toHaveLength(1);
  });
});

describe("partitionCluster", () => {
  it("keeps first `cap` by selection rule, rest overflow", () => {
    const a = item("a", 540, 30);  // 9:00
    const b = item("b", 540, 60);  // 9:00, longer → before a
    const c = item("c", 540, 30);  // 9:00, same as a → id order: a < c
    const d = item("d", 540, 30);
    const { visible, overflow } = partitionCluster([a, b, c, d], 3);
    expect(visible.map((i) => i.id)).toEqual(["b", "a", "c"]); // dur desc, then id
    expect(overflow.map((i) => i.id)).toEqual(["d"]);
  });
  it("empty overflow when within cap", () => {
    const { overflow } = partitionCluster([item("a", 0, 30)], 3);
    expect(overflow).toEqual([]);
  });
});
```

- [ ] **Step 2: 실행 → FAIL 확인**

Run: `cd crates/oxiline-app && bun run test`
Expected: FAIL — `Cannot find module '../timeline-math'`.

- [ ] **Step 3: 구현**

`src/lib/timeline-math.ts`:
```ts
import type { TimelineItem } from "../types";

/** Round `m` to the nearest `step` minutes; clamp into [0, 1440 - step]. */
export function snapMinute(m: number, step: number): number {
  const snapped = Math.round(m / step) * step;
  return Math.max(0, Math.min(1440 - step, snapped));
}

/** Clamp a duration so the block stays >= minDur and never ends past dayEndMin.
 *  Ceiling wins at the edge: when the window to dayEnd < minDur, the block ends
 *  exactly at dayEnd (returns the window) rather than forcing minDur past it.
 *  Spec §6: dayEnd ceiling is hard. Outer Math.min caps at dayEnd. */
export function clampDuration(
  start: number,
  dur: number,
  dayEndMin: number,
  minDur = 15,
): number {
  const maxDur = dayEndMin - start;
  return Math.min(Math.max(dur, minDur), maxDur);
}

/** Partition time-ranged items into maximal overlap clusters (start-asc). */
export function groupClusters(items: TimelineItem[]): TimelineItem[][] {
  const timed = items
    .filter((i) => i.start_minute != null && i.duration_minute != null)
    .slice()
    .sort((a, b) => a.start_minute! - b.start_minute!);
  const clusters: TimelineItem[][] = [];
  let runEnd = -1;
  for (const it of timed) {
    const start = it.start_minute!;
    const end = start + it.duration_minute!;
    if (clusters.length === 0 || start >= runEnd) {
      clusters.push([it]);
      runEnd = end;
    } else {
      clusters[clusters.length - 1].push(it);
      runEnd = Math.max(runEnd, end);
    }
  }
  return clusters;
}

/** Selection rule (spec §4): start asc, then duration desc, then id asc. */
function bySelection(a: TimelineItem, b: TimelineItem): number {
  if (a.start_minute! !== b.start_minute!) return a.start_minute! - b.start_minute!;
  const da = a.duration_minute!;
  const db = b.duration_minute!;
  if (da !== db) return db - da;
  return a.id < b.id ? -1 : a.id > b.id ? 1 : 0;
}

/** Split one cluster into `cap` visible (selection order) + the overflow tail. */
export function partitionCluster(
  cluster: TimelineItem[],
  cap: number,
): { visible: TimelineItem[]; overflow: TimelineItem[] } {
  const sorted = cluster.slice().sort(bySelection);
  return { visible: sorted.slice(0, cap), overflow: sorted.slice(cap) };
}
```

> **설계 결정 — 클러스터링은 items 기반 (lanes 아님):** `groupClusters`/`partitionCluster`는 `TimelineItem[]`를 받아 재클러스터링한다. `layout()`의 `Lane[]` 출력(`DayTimeline.tsx:13`)은 클러스터 소속을 잃고, `Lane.columns`(=`colEnds.length`)은 greedy-pack 열 수(동시중첩 깊이)라 클러스터 원소수와 다르기 때문이다. 그래서 lanes가 아니라 items에서 클러스터를 복원한다. `groupClusters`는 `layout()`의 inline 클러스터링(26-37행)과 **의도적으로 같은 interval 의미론**(다음 항목이 running max-end 미만이면 같은 클러스터)을 쓴다 — `layout()`을 건드리지 않는(출력 불변) 대신, 향후 `layout()`의 클러스터링이 바뀌면 `groupClusters`도 맞춰 유지할 것.
>
> **캡 의미론(명시, 둘이 다름):** 칩 트리거는 **item-count**(클러스터 원소수 > 3)다 — spec §4 "클러스터가 4개 이상"·"앞 3개 가시"에 부합. **column-count**(동시중첩 깊이 > 3)가 **아니다**. 엇갈리는 클러스터(절대 4개가 동시에 겹치지 않지만 원소수 4)에서 item-count는 칩을 띄우고 column-count는 띄우지 않는다. v1은 승인된 spec에 맞춰 **item-count**를 쓴다(위 staggered 테스트가 고정).

- [ ] **Step 4: 실행 → PASS 확인**

Run: `cd crates/oxiline-app && bun run test`
Expected: all tests pass (snap 2, clamp 2, cluster 1, partition 2 = 7).

- [ ] **Step 5: 커밋**

```bash
git add crates/oxiline-app/src/lib/timeline-math.ts crates/oxiline-app/src/lib/__tests__/timeline-math.test.ts
git commit -m "feat(app): pure timeline math (snap/clamp/cluster/partition)"
```

---

## Task 3: 감사 모크 브라우저 하네스  **[BROWSER-setup]**

`tauri-v2-browser-audit-mock` 스킬. Tauri/DB 없이 실제 React 컴포넌트를 브라우저에서 구동해 T4/T5/T6을 `page.mouse` + `__mockLog`로 검증. **스캐폴드 — T7에서 제거.**

**Files:**
- Create: `crates/oxiline-app/audit/mock.ts`
- Create: `crates/oxiline-app/audit/main.tsx`
- Create: `crates/oxiline-app/audit/index.html`

**Interfaces:**
- Produces: `http://localhost:1420/audit/index.html`에서 렌더링되는 실제 앱 + `window.__mockLog` (`{cmd, args}[]`).

- [ ] **Step 1: 시드/모크 작성**

`audit/mock.ts`:
```ts
// AUDIT-ONLY scaffolding. Patches window.__TAURI_INTERNALS__ so the real app
// boots in a plain browser without Tauri/DB. Removed before shipping (Task 7).
import type { TimelineItem, Category, Task } from "../src/types";

const DAY = "2026-07-31";

const categories: Category[] = [
  { id: "c-work", name: "업무", color_hue: 250, icon: "briefcase", sort_order: 0, is_builtin: true, created_at: "", updated_at: "" },
  { id: "c-study", name: "학습", color_hue: 300, icon: "book-open", sort_order: 1, is_builtin: true, created_at: "", updated_at: "" },
];

let timeline: TimelineItem[] = [
  { id: "t1", is_virtual: false, title: "아침 회의", start_minute: 540, duration_minute: 30, category_id: "c-work", is_done: false, is_skipped: false, origin_routine_block_id: null },
  { id: "t2", is_virtual: false, title: "코딩 세션", start_minute: 555, duration_minute: 60, category_id: "c-work", is_done: false, is_skipped: false, origin_routine_block_id: null },
  // 4-overlap cluster at 14:00 (for Task 6 verification)
  { id: "t3", is_virtual: false, title: "페어 프로그래밍", start_minute: 840, duration_minute: 60, category_id: "c-work", is_done: false, is_skipped: false, origin_routine_block_id: null },
  { id: "t4", is_virtual: false, title: "기사 읽기", start_minute: 840, duration_minute: 45, category_id: "c-study", is_done: false, is_skipped: false, origin_routine_block_id: null },
  { id: "t5", is_virtual: false, title: "블로그 초안", start_minute: 840, duration_minute: 60, category_id: "c-study", is_done: false, is_skipped: false, origin_routine_block_id: null },
  { id: "t6", is_virtual: false, title: "코드 리뷰", start_minute: 840, duration_minute: 30, category_id: "c-work", is_done: false, is_skipped: false, origin_routine_block_id: null },
];

const settings = { day_start_hour: 5, day_end_hour: 22, workload_warning_minutes: 600, locale: "ko", theme: "light" };

declare global {
  interface Window { __mockLog: { cmd: string; args: unknown }[]; }
}
window.__mockLog = [];

let nextId = 100;
const handlers: Record<string, (args: any) => unknown> = {
  get_settings: () => settings,
  is_onboarding_done: () => true,
  get_timeline: () => timeline,
  list_categories: () => categories,
  list_backlog: () => [],
  list_routines: () => [],
  get_now_context: () => ({ now: null, current: null, next: null }),
  get_week_report: () => ({}),
  create_task: (a) => {
    const t: Task = { id: `t${nextId++}`, date: a.date, title: a.title, category_id: a.categoryId, start_minute: a.startMinute, duration_minute: a.durationMinute, is_done: false, done_at: null, is_skipped: false, source: "manual", source_routine_block_id: null, notes: a.notes, sort_order: 0 };
    window.__mockLog.push({ cmd: "create_task", args: a });
    timeline.push({ id: t.id, is_virtual: false, title: t.title, start_minute: t.start_minute, duration_minute: t.duration_minute, category_id: t.category_id, is_done: false, is_skipped: false, origin_routine_block_id: null });
    return t;
  },
  update_task: (a) => {
    window.__mockLog.push({ cmd: "update_task", args: a });
    const ti = timeline.find((x) => x.id === a.id);
    if (ti) {
      if (a.startMinute != null) ti.start_minute = a.startMinute;
      if (a.durationMinute != null) ti.duration_minute = a.durationMinute;
    }
    return null;
  },
  materialize_if_virtual: (a) => a.id,
  set_task_done: (a) => { window.__mockLog.push({ cmd: "set_task_done", args: a }); return null; },
  set_task_skipped: () => null,
  delete_task: () => null,
};

let cbId = 0;
(window as any).__TAURI_INTERNALS__ = {
  invoke: (cmd: string, args?: Record<string, unknown>) => {
    if (cmd === "plugin:event|listen") return Promise.resolve(cbId++);
    const h = handlers[cmd];
    if (!h) return Promise.reject(new Error(`audit mock: unhandled command ${cmd}`));
    return Promise.resolve(h(args));
  },
  transformCallback: () => 0,
};
```

`audit/main.tsx`:
```ts
import "./mock";
import "../src/main";
```

`audit/index.html`:
```html
<!doctype html>
<html lang="ko">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>OxiLine audit</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="./main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 2: dev 서버 기동 + 브라우저 로드 확인**

Run: `cd crates/oxiline-app && bun run dev` (백그라운드; 포트 1420 고정).

브라우저 도구로 `http://localhost:1420/audit/index.html` 오픈 → `tab.observe()` 또는 `tab.screenshot()`.
Expected: 에러 없이 Day 타임라인 렌더, 시드 카드 6개(t1~t6) 표시. 콘솔에 "unhandled command" 없어야 함(있으면 해당 명령을 `handlers`에 추가).

- [ ] **Step 3: `__mockLog` 노출 확인**

브라우저 `tab.evaluate(() => (window.__mockLog).length)` → `0`.

- [ ] **Step 4: 커밋 (스캐폴드 명시)**

```bash
git add crates/oxiline-app/audit
git commit -m "chore(app): add audit/ browser mock harness (scaffolding, removed in final task)"
```

---

## Task 4: 블록 리사이즈 핸들 (하단 가장자리)  **[BROWSER]**

`BlockView`에 하단 리사이즈 그립 추가. 이동 드래그(dnd-kit)는 그대로, 그립의 `pointerdown`이 `stopPropagation`해 이동 발화 차단. 실시간 미리보기(로컬 state) → `pointerup`에서 `update_task({durationMinute})` 1회 커밋. 가상 occurrence는 먼저 `materializeIfVirtual`. `Shift+↑/↓` 키보드 증감.

**Files:**
- Modify: `crates/oxiline-app/src/components/BlockView.tsx` (현재 `Props` ~20행, `style` ~37행, `onKeyDown` ~62행, 렌더 `return` ~79행)

**Interfaces:**
- Consumes (T2): `snapMinute(m, step)`, `clampDuration(start, dur, dayEndMin, minDur)`.
- Consumes: `useUpdateTask()` (`hooks.ts:116`), `api.materializeIfVirtual(id)`.
- Props 추가: `dayEndMin: number`, `pxPerMin: number`.

- [ ] **Step 1: Props + 임포트 확장**

`BlockView.tsx` 상단 임포트에 추가:
```ts
import { useRef, useState } from "react";
import { api } from "../lib/api";
import { useUpdateTask } from "../hooks";
import { snapMinute, clampDuration } from "../lib/timeline-math";
```

`Props` 인터페이스에 필드 2개 추가 (`left`/`columns`/`top`/`height`/`past` 유지):
```ts
interface Props {
  item: TimelineItem;
  categories: Category[];
  left: number;
  columns: number;
  top: number;
  height: number;
  past: boolean;
  dayEndMin: number;   // 신규
  pxPerMin: number;    // 신규
}
```
컴포넌트 시그니처와 구조분해에 `dayEndMin`, `pxPerMin` 추가.

- [ ] **Step 2: 리사이즈 상태/핸들러 추가**

`BlockView` 본문(`const del = useDeleteTask();` 근처)에:
```ts
const upd = useUpdateTask();
const [previewDur, setPreviewDur] = useState<number | null>(null);
const drag = useRef<{ startY: number; startDur: number } | null>(null);

async function commitDuration(dur: number) {
  let id = item.id;
  if (item.is_virtual) id = await api.materializeIfVirtual(item.id);
  upd.mutate({ id, durationMinute: dur });
}

function onResizeDown(e: React.PointerEvent) {
  e.stopPropagation(); // dnd-kit 이동 드래그 발화 방지
  (e.currentTarget as Element).setPointerCapture(e.pointerId);
  drag.current = { startY: e.clientY, startDur: item.duration_minute ?? 30 };
}
function onResizeMove(e: React.PointerEvent) {
  if (!drag.current) return;
  const deltaMin = (e.clientY - drag.current.startY) / pxPerMin;
  const start = item.start_minute!;
  const rawEnd = start + drag.current.startDur + deltaMin;
  const dur = clampDuration(start, snapMinute(rawEnd, 15) - start, dayEndMin, 15);
  setPreviewDur(dur);
}
function onResizeUp(e: React.PointerEvent) {
  if (!drag.current) return;
  (e.currentTarget as Element).releasePointerCapture(e.pointerId);
  const dur = previewDur ?? drag.current.startDur;
  drag.current = null;
  setPreviewDur(null);
  commitDuration(dur);
}
```

- [ ] **Step 3: 효과 높이/라벨을 preview에 반영**

`style`의 `height`를 미리보기에 따라 바꾼다:
```ts
const effDur = previewDur ?? item.duration_minute ?? 0;
const effHeight = Math.max(effDur * pxPerMin, 22);
```
`style` 객체에서 `height: Math.max(height, 22)` → `height: effHeight` 로 교체. 본문 라벨의 `rangeLabel(item.start_minute, item.duration_minute)` → `rangeLabel(item.start_minute, previewDur ?? item.duration_minute)`, 그리고 `item.duration_minute` 배지 → `effDur`.

- [ ] **Step 4: 키보드 증감을 `onKeyDown`에 추가**

기존 `onKeyDown`의 `Backspace/Delete` 브랜치 뒤에:
```ts
} else if (e.shiftKey && (e.key === "ArrowUp" || e.key === "ArrowDown")) {
  e.preventDefault();
  const step = e.key === "ArrowUp" ? -15 : 15;
  const start = item.start_minute!;
  const dur = clampDuration(start, (item.duration_minute ?? 30) + step, dayEndMin, 15);
  commitDuration(dur);
  announce(t("a11y.resized", { n: dur }));
}
```
(`a11y.resized` 키는 T7에서 추가. 임시로 `announce(\`duration ${dur}\`)` 사용 가능.)

- [ ] **Step 5: 하단 그립 요소 렌더 추가**

`return` 안 가장 바깥 `<div>`의 자식 마지막(닫는 `</div>` 직전)에:
```tsx
<div
  role="separator"
  aria-orientation="horizontal"
  onPointerDown={onResizeDown}
  onPointerMove={onResizeMove}
  onPointerUp={onResizeUp}
  className="absolute bottom-0 left-0 right-0 flex h-2 cursor-ns-resize items-end justify-center pb-0.5 opacity-40 hover:opacity-100"
  style={{ touchAction: "none" }}
>
  <span className="h-[2px] w-6 rounded-full" style={{ background: "var(--color-text-subtle)" }} />
</div>
```
이 그립은 부모(카드 본문)의 `{...listeners}`(dnd-kit)보다 자식이므로, `onPointerDown`의 `stopPropagation`이 부모 리스너 도달을 막는다.

- [ ] **Step 6: `DayTimeline`에서 새 prop 전달**

`DayTimeline.tsx`의 `<BlockView>` 호출(~222행)에 추가:
```tsx
<BlockView
  /* 기존 prop 유지 */
  dayEndMin={dayEnd * 60}
  pxPerMin={pxPerMin}
/>
```

- [ ] **Step 7: 타입체크 + 브라우저 검증**

Run: `cd crates/oxiline-app && bun run build` → 에러 없음.

브라우저(audit): 시드 카드 t2(9:15, 60분)의 하단 그립을 `page.mouse`로 아래로 60px 드래그(`pxPerMin`=64/60이므로 약 56분 증분). 드래그 종료 후:
```js
tab.evaluate(() => window.__mockLog.filter((l) => l.cmd === "update_task"))
```
Expected: 정확히 **1**개의 `update_task`, `args.durationMinute` 가 약 115~120 (snap 15). 카드 높이가 늘어나 보임.

- [ ] **Step 8: 커밋**

```bash
git add crates/oxiline-app/src/components/BlockView.tsx crates/oxiline-app/src/components/DayTimeline.tsx
git commit -m "feat(app): drag-to-resize timeline blocks (bottom edge)"
```

---

## Task 5: 드래그로 만들면서 길이 지정 + 합성 클릭 억제  **[BROWSER]**

`DropZone`에 pointer 로직: 8px 이상 이동 시 "생성 중" 모드(고무줄), `pointerup`에서 길이 배지가 붙은 **컴포저** 오픈(태스크는 `Enter` 커밋 시 생성 — 기존 빠른추가와 동일). **`didDrag` ref**로 `pointerup` 후 합성 `click`이 컴포저를 이중 오픈하지 않게 단락. 순수 클릭은 기존 30분 컴포저.

**Files:**
- Modify: `crates/oxiline-app/src/components/DayTimeline.tsx` (`DropZone` ~322행, `adding`/`draft` state ~80행, 컴포저 렌더 ~236행, `<DropZone>` 호출 ~287행)

**Interfaces:**
- Consumes (T2): `snapMinute`, `clampDuration`.
- `DropZone` props 변경: `onAdd` → `onCompose(minute, durationMinute)`; 신규 `onPreviewChange`.

- [ ] **Step 1: `adding` state에 길이 추가**

`DayTimeline` 컴포넌트에서:
```ts
const [adding, setAdding] = useState<{ minute: number; durationMinute: number } | null>(null);
```
컴포저 `Enter` 핸들러의 `create.mutate({ ... durationMinute: 30 ... })` → `durationMinute: adding.durationMinute`. 컴포저에 길이 배지 표시(시간 라벨 옆):
```tsx
<span className="shrink-0 rounded-md px-1.5 py-0.5 font-mono text-[11px] font-medium"
  style={{ background: "var(--color-interactive-primary-subtle)", color: "var(--color-interactive-primary)" }}>
  {minuteToHHMM(adding.minute)} · {formatDuration(adding.durationMinute, lang as "ko" | "en")}
</span>
```
`<DropZone onAdd={(m) => setAdding({ minute: m, durationMinute: 30 })} />` → `onCompose={(m, d) => setAdding({ minute: m, durationMinute: d })}` 로 교체.

- [ ] **Step 2: 고무줄 미리보기 state + 렌더**

`DayTimeline`에:
```ts
const [creating, setCreating] = useState<{ startMin: number; curMin: number } | null>(null);
```
블록 렌더 영역(컴포저 바로 위)에:
```tsx
{creating && (
  <div className="pointer-events-none absolute left-0 right-0 z-[5] rounded-md border"
    style={{
      top: (Math.min(creating.startMin, creating.curMin) - dayStartMin) * pxPerMin,
      height: Math.abs(creating.curMin - creating.startMin) * pxPerMin,
      background: "color-mix(in oklch, var(--color-interactive-primary) 14%, transparent)",
      borderColor: "var(--color-interactive-primary)",
    }}>
    <span className="ml-1 font-mono text-[11px] font-medium" style={{ color: "var(--color-interactive-primary)" }}>
      {minuteToHHMM(Math.min(creating.startMin, creating.curMin))}–{minuteToHHMM(Math.max(creating.startMin, creating.curMin))}
    </span>
  </div>
)}
```
`<DropZone>`에 `onPreviewChange={setCreating}` 전달.

- [ ] **Step 3: `DropZone` 포인터 로직 + `didDrag` 억제 재작성**

`DropZone` 전체를 아래로 교체 (`onAdd` → `onCompose` + `onPreviewChange` + `dayEndMin`):
```tsx
function DropZone({
  dayStartMin, pxPerMin, date, heightPx, dayEndMin, onCompose, onHover, onPreviewChange,
}: {
  dayStartMin: number; pxPerMin: number; date: string; heightPx: number; dayEndMin: number;
  onCompose: (minute: number, durationMinute: number) => void;
  onHover: (minute: number | null) => void;
  onPreviewChange: (c: { startMin: number; curMin: number } | null) => void;
}) {
  const { setNodeRef, isOver } = useDroppable({
    id: "timeline-slot",
    data: { kind: "timeline-slot", date, pxPerMin, dayStartMin },
  });
  const creatingRef = useRef<{ startMin: number; startClientY: number; curMin: number } | null>(null);
  const didDragRef = useRef(false);

  function minuteAt(e: { clientY: number; currentTarget: unknown }): number {
    const rect = (e.currentTarget as HTMLDivElement).getBoundingClientRect();
    const y = e.clientY - rect.top;
    return Math.max(0, Math.min(1439, Math.round(y / pxPerMin + dayStartMin)));
  }

  function onPointerDown(e: React.PointerEvent) {
    const m = snapMinute(minuteAt(e), 15);
    creatingRef.current = { startMin: m, startClientY: e.clientY, curMin: m };
    didDragRef.current = false;
  }
  function onPointerMove(e: React.PointerEvent) {
    if (!creatingRef.current) { onHover(snap(minuteAt(e))); return; }
    if (!didDragRef.current && Math.abs(e.clientY - creatingRef.current.startClientY) > 8) {
      didDragRef.current = true;
    }
    if (didDragRef.current) {
      const m = snapMinute(minuteAt(e), 15);
      creatingRef.current.curMin = m;
      onPreviewChange({ startMin: creatingRef.current.startMin, curMin: m });
    }
  }
  function onPointerUp() {
    const c = creatingRef.current;
    creatingRef.current = null;
    if (c && didDragRef.current) {
      const start = Math.min(c.startMin, c.curMin);
      const end = Math.max(c.startMin, c.curMin);
      const dur = clampDuration(start, Math.max(15, end - start), dayEndMin, 15);
      onPreviewChange(null);
      onCompose(start, dur);
    }
    // 순수 클릭: didDragRef=false → onClick이 처리
  }
  function onClick(e: React.MouseEvent) {
    if (didDragRef.current) { didDragRef.current = false; onPreviewChange(null); return; } // 합성 클릭 억제
    onHover(null);
    onCompose(snapMinute(minuteAt(e), 15), 30);
  }

  return (
    <div
      ref={setNodeRef}
      className="absolute left-0 right-0 cursor-crosshair"
      style={{
        top: 0, height: heightPx, zIndex: 1, touchAction: "none",
        background: isOver ? "var(--color-interactive-primary-subtle)" : undefined,
        transition: "background var(--duration-slow) var(--ease-out)",
      }}
      onMouseMove={(e) => { if (!creatingRef.current) onHover(snap(minuteAt(e))); }}
      onMouseLeave={() => { if (!creatingRef.current) onHover(null); }}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onClick={onClick}
    />
  );
}
```
임포트에 `useRef`, `snapMinute`, `clampDuration` 추가. 기존 `minuteAt`/`snap` 사용은 유지(`snap`은 15분 슬롯).

- [ ] **Step 4: 타입체크 + 브라우저 검증**

Run: `bun run build` → 에러 없음.

브라우저(audit): 빈 영역(예: 11:00)에서 `page.mouse`로 아래로 약 90px 드래그 후 놓기. 그 후:
```js
tab.evaluate(() => ({
  creates: window.__mockLog.filter((l) => l.cmd === "create_task").length,
  last: window.__mockLog.find((l) => l.cmd === "create_task"),
}))
```
Expected: `creates` = **1**(컴포저가 `Enter` 없으면 0일 수 있음 — 그 경우 컴포저 입력에 텍스트 넣고 Enter 후 재평가). **핵심 단언:** 드래그-생성 직후 컴포저가 떠 있고 `create_task`가 **자동으로 발화하지 않음**(커밋-온-엔터). 인풋에 "테스트" 입력 + Enter 후 `creates` = 1, `last.args.durationMinute` ≈ 90분 영역.

합성 클릭 억제 단언: 같은 드래그 후 `document.activeElement`가 인풋이어야 하며(컴포저), `__mockLog`에 드래그 한 번으로 `create_task`/`update_task`가 불필요하게 2회 이상 발화하지 않음.

순수 클릭: 빈 영역 단순 클릭 → 30분 배지 컴포저 오픈(`didDragRef` false 경로).

- [ ] **Step 5: 커밋**

```bash
git add crates/oxiline-app/src/components/DayTimeline.tsx
git commit -m "feat(app): drag-to-create blocks + suppress synthetic click"
```

---

## Task 6: 겹침 소프트캡(3) 렌더 + "+N 더 보기" 칩  **[BROWSER]**

- `layout()`은 건드리지 않고(진짜 깊이만 계산), `DayTimeline` 렌더 레이어에서 클러스터별로 `partitionCluster` 적용 + 가시 항목의 `col`/`columns` 오버라이드. **원소수**>3 클러스터는 가시 3열(동일 폭) + 나머지는 우측 가장자리 "+N 더 보기" 배지(읽기 전용 팝오버). **캡 = item-count**(spec §4; column-count가 아님 — T2 메모·staggered 테스트 참고).

**Files:**
- Modify: `crates/oxiline-app/src/components/DayTimeline.tsx` (블록 렌더 루프 ~214행 `laid.map`)

**Interfaces:**
- Consumes (T2): `groupClusters`, `partitionCluster`.
- `Lane` 타입(`DayTimeline.tsx:13`)은 그대로.

- [ ] **Step 1: 캡 맵 계산 — overflowIds + col/columns 오버라이드**

`DayTimeline` 컴포넌트에서 `laid` 계산 직후, 캡(= item-count)을 적용할 두 맵을 만든다:
```ts
import { groupClusters, partitionCluster } from "../lib/timeline-math";

const timed = items.filter((i) => i.start_minute != null && i.duration_minute != null);
const { overflowIds, capOverride } = useMemo(() => {
  const ids = new Set<string>();
  const override = new Map<string, { col: number; columns: number }>();
  for (const cluster of groupClusters(timed)) {
    if (cluster.length > 3) {                         // ← item-count, NOT columns
      const { visible, overflow } = partitionCluster(cluster, 3);
      overflow.forEach((it) => ids.add(it.id));
      visible.forEach((it, idx) => override.set(it.id, { col: idx, columns: 3 }));
    }
  }
  return { overflowIds: ids, capOverride: override };
}, [items]);
```
**캡 = item-count**(`cluster.length > 3`). `columns`(동시중첩 깊이)가 **아니다** — 엇갈리는 4-원소 클러스터에서 max 동시중첩이 3이어도 칩이 뜬다(T2 staggered 테스트가 고정).
`capOverride`가 렌더 버그 방지의 핵심: `layout()`이 계산한 `col`/`columns`(예: columns=4)을 그대로 쓰면 가시 3개가 **1/4 폭 + 빈 칸**으로 깨지므로, 캡 클러스터의 가시 항목은 **선택 순서대로 col 0,1,2 + columns=3**으로 덮어쓴다(Step 2).

- [ ] **Step 2: 블록 루프 — 오버플로우 건너뛰기 + col/columns 오버라이드 적용**

기존 `laid.map(({ item, col, columns }) => {...})`에서 두 가지:
- 오버플로우 항목은 `null`(칩이 대체).
- 캡 클러스터의 가시 항목은 `capOverride`의 `col`/`columns=3`로 **덮어쓰기** — 안 하면 `layout()`의 `columns=4`가 남아 가시 3개가 1/4 폭 + 빈 칸으로 렌더됨(렌더 버그).
```tsx
{laid.map(({ item, col, columns }) => {
  if (overflowIds.has(item.id)) return null;            // 칩으로 대체
  const ov = capOverride.get(item.id);
  const effCol = ov?.col ?? col;
  const effColumns = ov?.columns ?? columns;
  const start = item.start_minute!;
  /* … 기존 top/height/past 계산 … */
  return (
    <BlockView key={item.id} item={item} categories={catsQ.data ?? []}
      left={effCol} columns={effColumns} top={top} height={height} past={past}
      dayEndMin={dayEnd * 60} pxPerMin={pxPerMin} />
  );
})}
```

- [ ] **Step 3: "+N 더 보기" 칩 렌더 (우측 가장자리 배지)**

가시 3열이 레인을 가득 채우므로(col 0,1,2 = 1/3씩) 별도 열 공간이 없다. 그래서 칩은 **우측 가장자리에 작은 배지로 오버레이**(z-4). `left:0, width:33%`는 블록 0과 겹쳐 보이므로 쓰지 않는다.
`DayTimeline`에 state 추가:
```ts
const [overflowOpen, setOverflowOpen] = useState<string | null>(null);
```
블록 루프 뒤에:
```tsx
{groupClusters(timed).filter((c) => c.length > 3).map((cluster) => {
  const { overflow } = partitionCluster(cluster, 3);
  const start = Math.min(...cluster.map((i) => i.start_minute!));
  const top = (start - dayStartMin) * pxPerMin;
  const chipId = `overflow:${start}`;
  return (
    <div key={chipId} className="pointer-events-auto absolute right-0 z-[4]" style={{ top }}>
      <button
        onClick={() => setOverflowOpen((id) => (id === chipId ? null : chipId))}
        className="rounded-full border border-border bg-surface-raised px-2 py-0.5 text-[11px] font-medium shadow-sm"
        style={{ color: "var(--color-text-muted)" }}
      >
        +{overflow.length} {t("timeline.more", { n: overflow.length })}
      </button>
      {overflowOpen === chipId && (
        <div className="absolute right-0 top-6 z-30 w-56 rounded-lg border border-border bg-surface-raised p-1 shadow-lg">
          <p className="px-2 py-1 text-[11px] font-semibold uppercase text-text-subtle">{t("timeline.overlapTitle")}</p>
          {overflow.map((it) => (
            <div key={it.id} className="flex items-center justify-between gap-2 rounded px-2 py-1 hover:bg-surface-sunken">
              <span className="truncate text-[12px]">{it.title}</span>
              <span className="shrink-0 font-mono text-[10px] text-text-subtle">{rangeLabel(it.start_minute, it.duration_minute)}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
})}
```

- [ ] **Step 4: 타입체크 + 브라우저 검증**

Run: `bun run build` → 에러 없음.

브라우저(audit): 시드 t3~t6(14:00, 4개 중첩) 렌더 확인. DOM/screenshot에서:
- **3개의 동일 폭 열이 레인을 빈틈없이 채움**(columns=3, col 0/1/2). 1/4 폭이나 col 3 빈칸이 보이면 `capOverride` 누락 버그 → Step 1/2 재확인.
- 선택 규칙(시작 840 동점 → 길이 내림차순): 가시 = t3(60·페어 프로그래밍), t5(60·블로그 초안), t4(45·기사 읽기); **오버플로우 1개** = t6(30·코드 리뷰) → 우측 가장자리 "+1 더 보기" 배지.
- 배지 클릭 → 팝오버에 "코드 리뷰 14:00-14:30" 표시. 재클릭 → 닫힘.

`screenshot`으로 위 4가지 시각 확인(특히 **"동일 폭 3열, 빈칸 없음"** — `capOverride`가 제대로 덮어썼는지의 결정적 증거).

- [ ] **Step 5: 커밋**

```bash
git add crates/oxiline-app/src/components/DayTimeline.tsx
git commit -m "feat(app): overlap soft-cap 3 + overflow chip"
```

---

## Task 7: i18n 문자열 + 최종 타입체크 + audit/ 제거  **[UNIT+build]**

신규 가시 문자열 키 추가, 전체 빌드 게이트 통과, 검증 스캐폴드(`audit/`) 제거.

**Files:**
- Modify: `crates/oxiline-app/src/locales/ko.json`, `locales/en.json`
- Remove: `crates/oxiline-app/audit/` (스캐폴드)

**Interfaces:**
- Produces: `bun run build` clean; 프로덕션 빌드 입력은 `{main, hud}` 그대로.

- [ ] **Step 1: i18n 키 추가 (ko/en)**

`locales/ko.json`의 `"timeline"` 블록에:
```json
"more": "+{{n}} 더 보기",
"overlapTitle": "이 시간의 다른 일정"
```
`locales/en.json`의 `"timeline"` 블록에:
```json
"more": "+{{n}} more",
"overlapTitle": "Other items at this time"
```
양쪽 `"a11y"` 블록에 (T4에서 사용):
```json
"resized": "길이를 {{n}}분으로 바꿨어요"   // ko
"resized": "Duration set to {{n}} minutes"  // en
```

- [ ] **Step 2: 빌드 게이트**

Run: `cd crates/oxiline-app && bun run build`
Expected: `tsc -b` + `vite build` 에러/경고 없음.

- [ ] **Step 3: 단위 테스트 회귀**

Run: `cd crates/oxiline-app && bun run test`
Expected: Task 2 테스트 전부 PASS (회귀 없음).

- [ ] **Step 4: audit/ 스캐폴드 제거**

```bash
rm -rf crates/oxiline-app/audit
git add -A crates/oxiline-app
```
`vite.config.ts`의 `rollupOptions.input`은 `{main, hud}` 그대로이므로 빌드 입력 변동 없음 확인.

- [ ] **Step 5: 최종 빌드 재확인 + 커밋**

Run: `cd crates/oxiline-app && bun run build` → 여전히 clean.

```bash
git add crates/oxiline-app/src/locales crates/oxiline-app/audit
git commit -m "feat(app): i18n for resize/overlap + remove audit scaffolding"
```

---

## Self-Review (plan 작성자 점검)

**1. 스펙 커버리지:**
- §2 리사이즈 핸들(하단, 실시간 미리보기, 커밋, 클램프 15/종료시각, 가상 materialize, Shift+↑/↓) → **Task 4**. ✓
- §3 드래그-생성(8px 임계치, 고무줄, 커밋-온-엔터 컴포저, 클릭=30분) → **Task 5**. ✓
- §4 소프트캡 3 + 선택 규칙 + 읽기전용 팝오버, `layout()` 순수 유지 → **Task 6**(렌더 레이어) + **Task 2**(순수 함수). ✓
- §5 `didDrag` 합성 클릭 억제 → **Task 5** Step 3의 `onClick` 단락. ✓
- §6 불변식(15분 바닥, 종료시각 상한, 가상 materialize) → **Task 2** `clampDuration` + **Task 4/5** 적용. ✓
- §7 스냅(이동 5 / 생성·리사이즈 15) → **Task 2** `snapMinute(step)` + **Task 4/5**에서 15 사용. ✓
- §8 백엔드 변경 없음 → 모든 Task가 `src/**`에만 머뭄. ✓
- §11 검증(audit-mock + `__mockLog`) → **Task 3** 하네스 + Task 4/5/6의 [BROWSER] 단계. ✓

**2. 자리표시자 스캔:** "TODO"/"TBD"/"적절히" 없음. 모든 코드 단계에 실제 코드. ✓

**3. 타입 일관성:** `snapMinute`/`clampDuration`/`groupClusters`/`partitionCluster` 시그니처가 Task 2 정의와 Task 4/5/6 사용처에서 일치. `onCompose(minute, durationMinute)` 가 Task 5 정의·사용 양쪽 일치. `dayEndMin`/`pxPerMin` prop가 Task 4 추가·`DayTimeline` 전달 양쪽 일치. ✓

**4. 주의사항(구현자에게):**
- Task 2 `clampDuration`은 ceiling-wins(`Math.min(Math.max(dur, minDur), maxDur)`) — dayEnd 임박 시 minDur보다 짧아도 dayEnd를 넘기지 않는다(spec §6). 테스트 기대값 `10`이 이 동작을 고정하므로 구현·테스트 불일치 없음.
- Task 3의 감사 모크는 앱이 호출하는 모든 명령을 커버해야 함 — 브라우저 콘솔에 `unhandled command`가 뜨면 해당 명령을 `handlers`에 추가(현재 Day/Header 경로에서 쓰는 명령은 모두 포함됨).
- `pxPerMin`=64/60은 하드코딩(`DayTimeline.tsx:88`) — 리사이즈/고무줄 계산도 동일 값 사용.
