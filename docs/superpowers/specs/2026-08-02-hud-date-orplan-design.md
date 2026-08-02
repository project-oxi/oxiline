# HUD 보강 · 날짜 popover · OR-계획 다중선택+리사이즈 — 설계

> 날짜: 2026-08-02
> 범위: recording-네이티브 UI 3종 보강. 레거시(Task 8 대상: Backlog/Week/Report/RoutineManager)와 무관하게 동작한다.
> 선행: Plan 2 Task 1–7 완료(3-pane shell, recording commands, two-lane timeline, 사이드바, ActivitySwitcher ⌘⇧A, 단일 activity 드래그→create_plan).

## 0. 개요

세 기능은 모두 recording 레이어 위에서 동작하며, 기존 `PlanSlot`/`RecordState`/`Compliance` 데이터 모델을 그대로 소비한다. 레거시 `NowContext`/`Task`/`RoutineBlock`에는 의존하지 않는다.

| 기능 | 주요 파일 | 데이터 소스 |
|---|---|---|
| A. HUD 보강(⌘⇧O) | `hud.tsx` | `RecordState` + `PlanSlot` + `Compliance(week)` |
| B. 날짜 popover | `components/Header.tsx` | `useTimelineRange(월 범위)` |
| C. OR-계획 다중선택+병합+리사이즈 | `Sidebar.tsx`, `lib/dnd.tsx`, `RecordTimeline.tsx` | `createPlan`/`add_option`/`resize_plan` |

### 0.1 백엔드 변경 (공통)

코어 `plan.rs`에는 이미 `add_option(plan_id, activity_id)`/`remove_option`이 구현되어 있다(§plan.rs:195–238). 이들이 tauri 명령으로만 미노출된 상태. 리사이즈만 코어 신규 함수가 필요하다.

**코어 신규** (`crates/oxiline-core/src/plan.rs`):
```rust
/// duration_minute만 부분 갱신. start/weekday/title/options는 그대로.
pub fn resize_plan(conn: &Connection, id: &str, duration_minute: u16) -> Result<Plan>;
```
- `update_plan`은 `PlanInput` 전체를 요구(start_minute/weekday_mask 직접 할당)해 리사이즈 단독엔 부적합 → 전용 부분 갱신.
- 트랜잭션 내에서 `duration_minute` 컬럼만 `UPDATE`. 최소값 검증은 호출측(프론트 스냅/최소 15분)에서도 하되, 코어에서도 `duration_minute == 0` 거부.

**app 명령 신규** (`src-tauri/src/commands.rs` + `lib.rs` invoke_handler 등록):
```rust
#[tauri::command] #[specta::specta]
pub fn add_plan_option(state, planId: String, activityId: String) -> Result<PlanOption, String>;

#[tauri::command] #[specta::specta]
pub fn resize_plan(state, planId: String, durationMinute: u16) -> Result<Plan, String>;
```
- `add_plan_option` = 기존 `plan::add_option` 래핑.
- 명령 인자는 Tauri JS 바인딩 컨벤션(camelCase) 준수.

**프론트 api.ts / types.ts**:
- `api.addPlanOption(planId, activityId)` / `api.resizePlan(planId, durationMinute)`
- `PlanOption` 타입 추가(`plan_id`, `activity_id`, `sort_order`) — `add_option` 반환용. 병합 후엔 슬롯 refetch가 주이므로 반환값 사용은 선택.

---

## A. HUD 녹화-네이티브 전면 재구성

### A.1 현황과 문제
`hud.tsx`는 레고시 `NowContext`(`onNowUpdate` → `get_now_context`, routine/task 기반 now/next)만 표시한다. 녹화 레이어(실시간 세션·주간 목표)를 전혀 반영하지 않는다 — recording이 앱의 본체임에도 글로벌 오버레이가 가장 중요한 정보를 놓친다.

### A.2 설계
데이터 소스 전환: `onNowUpdate`(레거시 이벤트) 제거 → `useRecordState()` + `useSlots(today)` + `useCompliance("week")` (react-query, `onDbChanged`로 자동 갱신).

레이아웃(컴팩트 카드, 2초 글랜스용):
```
┌───────────────────────────────┐
│ [OxideBar — 오늘 기록 분포]     │   ← 유지(요일 스트립)
│                               │
│ ● 녹화중 · 코딩                │   ← active 세션(현실)
│ 0:40 경과 · 주간 8h20m/20h     │
│ [▓▓▓▓░░░░░░] 42%              │   ← 활동 주간 컴플라이언스
│                               │
│ 다음 · 독서 · 14:00 (1h 후)    │   ← 다음 PlanSlot(의도)
└───────────────────────────────┘
```

분기:
- **녹화 중**(`state.active != null`): `● {activity.name}` + 경과(`elapsed_seconds` → `hmm`) + 그 활동의 주간 컴플라이언스(`Compliance(week)`에서 `activity.id`로 찾기 → `recorded/target` + 막대, `hmm`/`complianceLabel` 재사용).
- **자유 시간**: 현재 시각이 어떤 `PlanSlot`의 `[start, start+duration)`에 들어가면 `지금 예정 · {첫 옵션명}`(다중이면 `… OR`), 아니면 `지금 · 자유 시간`.
- **다음**: now 이후 첫 `PlanSlot`. `{첫 옵션명 OR} · {hhmm} · ({X}분 후)`. 없으면 생략(녹화 중이어도).
- 의도(다음 계획)·현실(녹화) 나란히 → 기존 "의도+현실" 목업 테마 일관.

### A.3 갱신
HUD webview는 영구(supended 없이 show/hide). 표시 시점 최신 데이터 보장: `visibilitychange`(또는 Rust 측 show 훅)에서 관련 query `refetch`. react-query `staleTime` 기본값으로도 충분하지만, 세션 경과·now-line은 표시 순간 값이어야 한다.

### A.4 검증
- 녹화 중 ⌘⇧O → `● 활동 · 경과 · 주간 N/M + 막대` 표시(1).
- 미녹화 + 현재 슬롯에 계획 → `지금 예정 · 활동`(2).
- 미녹화 + 자유 → `자유 시간` + `다음 …`(3).
- 2초 후 사라짐 유지, 텍스트 입력 커서 유지(기존 권한 동작 회귀 없음)(4).

---

## B. 날짜 popover — 월 달력 + 기록 마커

### B.1 현황과 문제
`Header.tsx`의 날짜 제목은 `goToToday` 버튼이고, 좌우 chevron·주간 스트립(현재 주 7일)만 있다. 임의 날짜(예: 2주 전 회고)로 점프할 수 없다.

### B.2 설계
날짜 제목 버튼 → 클릭 시 버튼 하단에 anchored popover.
```
        ‹  2026년 8월  ›       [오늘]
  월 화 수 목 금 토 일
              ·  1  2  3
   ●  5  6  7  8  9 10
  11 12 13 14 …
```
- 월~일 컬럼(주간 스트립과 정렬). `‹ ›`로 월 이동. 날짜 클릭 → `setDate(dStr)` + `setView("today")` + 닫기.
- **마커**: `useTimelineRange(월 첫날, 월 마지막날)`로 각 날의 아이템 category hue 집계 → 최대 5개 색 점. 주간 스트립(`Header.tsx:107` 기존 로직)과 동일 알고리즘 재사용.
- 오늘 = 채운 원(강조색), 선택일 = 링, 다른 달 날짜 = 흐리게.
- `[오늘]` 버튼 = 즉시 오늘 점프 + 닫기.
- 닫기: `Escape` / 외부 클릭 / 날짜 선택.

### B.3 구현
- 경량 커스텀 popover: 트리거 버튼 ref 기준 절대 위치 + `onPointerDown` outside-click + `Escape` 핸들(이미 `useGlobalKeys`에 Escape 있음 — popover 열림 우선 처리).
- 라이브러리(radix 등) 추가 지양. 프로젝트의 `Modal.tsx` 패턴(포털 + outside)을 popover용으로 축소.
- 접근성: `role="dialog"` + `aria-label`. 그리드 화살표 키 이동은 2단계(기본 클릭 우선).

### B.4 검증
- 제목 클릭 → 이번 달 그리드 + 오늘 강조 + 각 날 색 점(1).
- `‹ ›` → 월 이동, 마커 갱신(2).
- 날짜 클릭 → 해당일 today 뷰 + popover 닫힘(3).
- 외부 클릭/Escape → 닫힘(4).

---

## C. OR-계획 다중선택 + 드롭-병합 + 리사이즈

### C.1 다중선택-드래그 (라이브러리 → 타임라인)
사이드바 활동 카드(`Sidebar.tsx` `DraggableActivity`)에 선택 모델 추가:
- **제스처**(확정): 카드 클릭 = 단일 선택 토글(하이라이트), `⌘`/`Ctrl`+클릭 = 다중 추가, 빈 곳 클릭 = 해제. 선택 상태는 `ActivityLibrary` 지역 state(`Set<activityId>`).
- 선택 카드 = 링/테두리 하이라이트. 드래그 시 `DragOverlay`에 `N` 배지.
- 드래그 페이로드 변경: `data: { kind: "activity", activityIds: string[] }` (선택 전체; 미선택 클릭-드래그 시 클릭한 1개).
- `dnd.tsx` `handleDragEnd`: `kind === "activity"` → 드롭존이 **타임라인**이면 `createPlan({ activity_ids: activityIds, … })` = OR 계획(여러 개면). 단일이면 종래 단일 계획.

드래그 개시 vs 클릭 선택 구분: `PointerSensor` activation `distance: 8` 유지 → 8px 미만은 클릭(선택 토글), 이상은 드래그(선택된 전체 운반).

### C.2 드롭-병합 (기존 계획에 옵션 추가)
- `RecordTimeline.tsx` `PlanLane`의 각 카드를 `useDroppable({ id: "plan-"+planId, data: { kind: "plan-slot", planId } })`로 등록.
- 드롭 충돌 우선순위: `DndContext` collisionDetection을 `rectIntersection`으로 변경(기본 `pointerWithin`은 큰 타임라인이 작은 카드를 덮어버림) → 가장 많이 겹치는 droppable이 승자.
- `handleDragEnd` 분기:
  - `over.kind === "plan-slot"` → `addPlanOption(planId, activityId)` (단일 activity 추가). 다중 선택 드래그를 카드에 올린 경우: 첫 activity를 추가하고 나머지는 무시(OR 카드는 단일 옵션 추가가 자연스럽다) — 또는 전부 추가. **확정**: 다중이어도 전부 추가(일관성).
  - `over.kind === "timeline-slot"` → 신규 계획(C.1).
- 시각: 병합 성공 시 해당 슬롯 refetch → 옵션 점 증가.

### C.3 리사이즈 (계획 카드 크기조절)
- 각 계획 카드 하단 가장자리에 핸들(높이 ~6px, 커서 `ns-resize`).
- **표시**(확정): hover 시에만 노출(평소 카드는 깔끔).
- 포인터 드래그로 구현(dnd-kit과 분리 — 리사이즈는 카드 내 1D 수직 드래그; `onPointerDown`에서 캡처, `pointermove`로 `duration_minute` 가시 갱신, `pointerup`으로 확정).
- 스냅: `SNAP_MINUTES`(5분). 최소 15분, 최소 미만 드래그 시 15분으로 클램프.
- 확정 시 `resizePlan(planId, durationMinute)` → 슬롯 refetch.
- OR 표시(`OR` 배지)/옵션 점 렌더는 유지; 핸들만 추가.

### C.4 상호작용 충돌 회피
- 리사이즈 핸들의 `onPointerDown`에서 `e.stopPropagation` → dnd-kit 드래그 개시 차단.
- 계획 카드는 현재 드래그 이동(재배치)을 지원하지 않는다(범위 밖, §D). 따라서 카드 전체는 dnd-kit draggable이 아니다 — 핸들(리사이즈) + 카드 전체(드롭 타겯)만.

### C.5 검증
- 활동 2개 ⌘클릭 선택 → 타임라인 드롭 → OR 계획(옵션 2개) 생성(1).
- 활동 1개 드래그 → 기존 계획 카드 위 드롭 → 옵션 1개 추가(2).
- 카드 하단 핸들 드래그 → 5분 스냅으로 높이 변화 → 놓으면 duration 갱신(3).
- 핸들 드래그 중 타임라인 드롭존이 반응하지 않음(stopPropagation)(4).

---

## D. 명시적 범위 밖 (YAGNI)
- **계획 카드 이동**(start_minute 변경): 요청에 없음. 자연스러운 후속.
- **계획 삭제 / 옵션 제거 UI**: `delete_plan`/`remove_option`은 코어에 있으나 이번 범위 아님.
- **popover 무한 스크롤**: `‹ ›` 네비로 충분.
- **HUD now/next 백엔드 명령**: 클라이언트가 `slots_for_date`에서 도출하므로 불필요.

## E. 영향 받는 파일
- 코어: `crates/oxiline-core/src/plan.rs`(`resize_plan` 신규 + 테스트)
- app Rust: `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`(명령 2개 등록)
- 프론트: `lib/api.ts`, `types.ts`, `hooks.ts`(mutation 2), `hud.tsx`, `components/Header.tsx`, `components/Sidebar.tsx`, `lib/dnd.tsx`, `components/RecordTimeline.tsx`, `styles.css`(popover/핸들/선택 스타일)

## F. 위험
- **드롭 충돌**: 큰 타임라인 droppable 안의 작은 카드 droppable. `rectIntersection` + 드롭존 z-order/면적 기반 선택이 필수. 검증 C.5(2)(4).
- **HUD webview 갱신**: 영구 webview의 stale 데이터. `visibilitychange` refetch로 완화(A.3).
- **선택 vs 드래그**: `distance: 8` activation이 클릭 선택과 드래그를 분리(C.1). 8px 임계값은 기존값 유지.
- **`update_plan` 직접 할당 함정**: 리사이즈를 `update_plan`으로 하면 start/weekday가 덮어씌워질 위험 → 전용 `resize_plan`으로 회피(0.1).
