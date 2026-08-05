# 09. UI 명세(Specification) — OxiLine 메인 창

> **지위:** 본 문서는 2026-08-05 현재 `main`에 구현된 메인 창 GUI의 **최종 UI·인터랙션 명세**다.
> 원본 그린필드 설계서(`01`~`08`)의 철학과 토큰 체계를 전제로, 화면에 도달한 최종 형태를
> 단일 진실 공급원(single source of truth)으로 고정한다. 진단 이력은 §13 부록에 압축 보존.
>
> 단위: 본 명세는 **동작과 구조**를 기술하며, 라인 번호는 drift되므로 **컴포넌트/함수명**으로
> 구현을 인용한다(§12). 디자인 토큰 정의는 `doc/06-design-system.md` + `src/tokens/*`이 우선.

---

## 9.1 설계 원칙 (재확인, 변경 없음)

1. **표면이 곧 인터페이스다.** 생성·녹화·이동·삭제·크기조절은 모두 타임라인/사이드바 표면에서
   직접 일어난다. 모달(⌘K 커맨드 팔레트, ⌘⇧A 활동 스위처)은 보조 경로다.
2. **시간이 주인공이다.** 타임라인(중앙)이 가장 밝은 표면(`--color-surface`), 양 옆 pane은
   한 톤 낮다. 산화 바는 메인 창 헤더에 항상 존재한다(§6.6).
3. **보이는 것이 작동한다.** 드롭 영역은 `isOver` 링으로, 녹화 가능 상태는 항상 보이는
   컨트롤(히어로 필)로 드러난다. 숨겨진 제스처에 동작을 맡기지 않는다.

---

## 9.2 정보 구조 & 표면 위계

3-tier 표면 토큰(`surface-sunken / surface / surface-raised`)으로 pane을 위계화한다.

```
┌───────────────────────────────────────────────────────────────────────┐
│ ●●●   ‹  8월 5일 화 ⌄  ›   [월3][화4][수5][목6][금7][토8][일9]   ⏺녹화  🔍 ⚙  │ ← Header (surface-raised) + 산화 바
├───────────────┬──────────────────────────────────┬────────────────────┤
│   Sidebar     │       Timeline (주인공)            │     Inspector       │
│   surface-    │       surface (가장 밝음)           │   surface-raised    │
│   sunken(오목) │                                   │   + 좌측 hairline   │
│   +우측 hair   │   now-line(양 레인 관통)           │                     │
├───────────────┴──────────────────────────────────┴────────────────────┤
│                         (오버레이: ⌘K / ⌘⇧A / ⌘, / 온보딩)               │
└───────────────────────────────────────────────────────────────────────┘
```

| 영역 | 표면 토큰 | 보더 | 의미 |
|---|---|---|---|
| Header | `surface-raised` | 하단 `1px border` | 크롬. 날짜·요일·녹화 트랜스포트 |
| Sidebar | `surface-sunken` | 우측 `1px border` | 보조/수납. "여기서 가져와 타임라인에 놓는다" |
| Timeline | `surface` (기본, 가장 밝/크) | — | 주인공. 시간이 흐르는 무대 |
| Inspector | `surface-raised` | 좌측 `1px border` | 요약/통계 |

---

## 9.3 헤더 — 단일 커맨드 바 + 산화 바 스트립

> 구현: `src/components/Header.tsx`. 3행 구조에서 **1행(44px) + 산화 바 스트립(12px)**로 압축해
> 타임라인에 약 85px을 반환했다. 행 전체가 창 드래그 영역(`data-tauri-drag-region`);
> 인터랙티브 컨트롤은 드래그 면제 아일랜드(`button`/`a`/`input`).

### 9.3.1 레이아웃 (단일 행)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ ●●●  ‹ [8월 5일 화 ⌄] ›   [월3][화4][(수5)][목6][금7][토8][일9]   [⏺ 녹화] 🔍 ⚙ │
├─────────────────────────────────────────────────────────────────────────────┤
│ ▁▂▃▅▆▇▆▅▃▁  ●(now)  ▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁ │ ← 산화 바
└─────────────────────────────────────────────────────────────────────────────┘
```

좌→우 4 클러스터(`pl-[56px]`로 트래픽 라이트 피함):
1. **날짜 마스트헤드** — `‹` / `8월 5일`(17px bold tabular) + `화`(12px semibold) + 연도(연도가
   올해가 아닐 때만) + `ChevronDown` / `›`. 좌우 셰브론은 1일 이동(`shiftDate(±1)`).
2. **요일 칩(Week strip)** — `DayChip` 컴포넌트 7개(월–일). 상세는 §9.3.2.
3. **(spacer, `flex-1`)** — 클러스터 간 여백.
4. **우측 클러스터** — 녹화 히어로 필(§9.7) + `⌘K` 검색 + `⚙` 설정.

### 9.3.2 요일 칩 (`DayChip`)

각 칩(`flex-1`, `h-8`)은 한 날짜를 나타내며, 흩어진 hue 점 대신 **마이크로 산화 바**로 그 날의
실제 기록을 압축 표현한다(앱 시그니처와 통일).

```
┌──────┐
│ 수 5 │   ← 요일 라벨(10px) + 날짜 숫자(12px semibold tabular)
│ ▂▄▆▃ │   ← 마이크로 산화 바(3px, w-6): 그 날 기록의 hue 세그먼트
└──────┘
```

- **선택일** = `bg-interactive-primary` 채워진 칩 + `--shadow-today-node` 글로우; 라벨/숫자는
  `interactive-primary-foreground`. 마이크로 바 track은 `foreground/25`.
- **오늘(선택 아님)** = `bg-interactive-primary-subtle` 틴트 + primary 컬러 라벨/숫자.
- **그 외** = 투명; 호버 시 `bg-surface-sunken`.
- 마이크로 바 세그먼트는 `isoLocal(record).minute`를 `day-window` 비율로 환산(`useMemo`).

### 9.3.3 캘린더 팝오버 — body 포털

날짜 마스트헤드 버튼(`aria-haspopup="dialog"`) 클릭 → 월 그리드 팝오버. **`createPortal`로
`document.body`에 렌더**하여 타이틀바 `data-tauri-drag-region`의 영향권에서 완전 이탈(과거 P5
드래그-리전 간섭 이슈 해소). 날짜 버튼 `getBoundingClientRect()` 기반 `position: fixed`로
버튼 바로 아래에 고정(우측 클램프 포함).

- 헤더 행: `‹` / `2026년 8월` / `›` / `오늘`(빠른 복귀).
- 요일 헤더 `월…일`, 일별 셀(일자 + hue 점 최대 5개). 오늘=`bg-interactive-primary`,
  선택=`ring-2 ring-interactive-primary`.
- 닫기: 외부 클릭(`pointerdown`) / `Escape` / 날짜 선택 / 창 리사이즈.

### 9.3.4 산화 바 스트립 (`OxideBar`)

헤더 하단 풀폭, 높이 10px(`height` prop). 선택일의 실제 기록을 hue 세그먼트로 압축.
클릭 → `useUi.requestScroll(minute)` → 타임라인이 해당 분으로 스크롤(`OxideBar.onClickMinute`).
오늘인 경우에만 now 마커 표시(`showNow`). HUD는 `compact` 모드로 별도 사용.

---

## 9.4 타임라인 (`RecordTimeline`)

> 구현: `src/components/RecordTimeline.tsx`. 두 레인(계획/실제) 공유 로컬 wall-clock 시간축.

### 9.4.1 구조

- **모드 토글** `[계획 | 실제 | 둘 다]` — 세그먼티드 컨트롤. 활성 탭 =
  `bg-surface-raised` + `ring-border-strong` + `--shadow-sm`.
- **시간축 gutter**(`w-12`) — 시각 라벨(10px tabular), 우측 정렬.
- **레인 컨테이너** — `both` 모드 시 `grid-template-columns: 1fr 1fr`; 사이드바 드래그
  드롭 타겟(`useDroppable id="record-timeline"`, `isOver` 시 `ring-interactive-primary/40`).

### 9.4.2 계획 레인 (`PlanLane` + `PlanCard`)

`PlanSlot[]`을 `PlanCard`로 렌더. 각 카드는 **4가지 직접 조작**을 한 표면에 공유한다:

| 동작 | 제스처 | 구현 | 커밋 |
|---|---|---|---|
| **생성(빠른 추가)** | 빈 공간 클릭 | `PlanLane.onPointerDown` → 30분 초안 `DraftBlock` | `createPlan` |
| **생성(고무줄)** | 빈 공간 드래그(>6px) | `rubber` 상태 → `DraftBlock` | `createPlan` |
| **크기 조절** | 카드 하단 1.5px 핸들 드래그 | `resizeDuration` + 5분 스냅 | `resizePlan` |
| **이동** | 카드 본문 드래그 | 포인터 캡처 + 5분 스냅 + day-window 클램프 | `updatePlan` (start만) |
| **삭제** | 카드 호버 `×` 버튼 | `useDeletePlan` | `deletePlan` |
| **OR 옵션 추가** | 사이드바 활동 드래그→카드 | `useAddPlanOptions` (dnd-kit) | `addPlanOptions` |

- **PlanCard 시각**: 점선 아웃라인(`border-dashed border-border-strong`), 호버 시
  `--shadow-md`; 이동 중 `z-30` + `--shadow-lg` + `cursor-grabbing`. 헤더 행에
  `HH:MM · Nm` + (OR 마커) + (이동 중 `→ 새시각`) + 호버 `×`.
- **이동 보존**: `update_plan`은 `weekday_mask`를 직접 재할당(부분 아님)하므로, `PlanSlot`이
  `weekday_mask`를 carry하고 `useMovePlan`이 기존 mask/duration을 그대로 전달한다
  (`date: null, title: null, activity_ids: []` → 각각 보존). 반복 계획도 반복 일정이 유지.

### 9.4.3 초안 블록 (`DraftBlock`) — 인라인 에디터

빈 공간 클릭/드래그로 나타나는 카드. "폼"이 아니라 **카드 자체가 에디터**:

```
┌──────────────────────────────────┐
│ 14:30–16:00  90분          ⏎ esc │  ← 헤더 행: 시간 범위 배지 + 기간 + kbd 힌트
│ 활동 이름                         │  ← 보더리스 인플레이스 입력(13px medium)
└──────────────────────────────────┘
```

- `border-interactive-primary/70` + `--shadow-lg` + `bg-surface-raised`.
- 헤더 행: `bg-interactive-primary-subtle` 시간 범위 배지(mono tabular) + 기간 + 우측
  `⏎`/`esc` kbd(인라인, 세로 여백 차지 않음).
- 입력: 보더리스·투명 배경, placeholder `활동 이름`. `Enter`=커밋, `Esc`=취소,
  blur 시 값이 있으면 커밋.
- 커밋(`commitDraft`): 매칭 활동이 있으면 그 활동으로, 없으면 `createActivity` 후 `createPlan`.

### 9.4.4 실제 레인 (`ActualLane`)

`ActivityRecord[]`를 hue 채운 블록으로. **활동 이름 표시**(`nameById` 맵; 과거 raw id 버그 수정),
좌측 3px hue 레일, 라이브 세션은 펄스 점. now-line은 별도(§9.4.5).

### 9.4.5 now-line (`NowLine`)

래퍼 레벨에 렌더되어 **양 레인을 관통**(과거 실제 레인 전용 → 확장). `border-status-error` 가로선 +
우측 끝 모노 시각 칩(`bg-status-error text-text-inverse`). 오늘 + 현재 분이 day-window 내일 때만.

---

## 9.5 사이드바 (`Sidebar`)

> 구현: `src/components/Sidebar.tsx`. `aside w-[260px] surface-sunken`.

1. **NowCard** — 활성 세션: hue 보더 카드 + 펄스 점 + 활동명 + 경과 시간 + `⏸ 멈춤`.
   비활성: `▶ 녹화 시작` CTA(직전 활동 즉시 시작, 없으면 스위처 오픈) + 단축키 힌트.
2. **ActivityLibrary** — `＋ 추가` 인라인 입력으로 활동 생성(빈 상태 = `＋ 첫 활동 만들기` CTA).
   각 `DraggableActivity`: hue 점 + 이름 + 주간 목표 + neutral 바(목표 틱 포함) +
   `남음/달성/초과` 라벨. 호버 시 `▶` 빠른 녹화 버튼. dnd-kit 드래그 소스(다중 선택 지원).

---

## 9.6 인스펙터 (`Inspector`)

> 구현: `src/components/Inspector.tsx`. `aside w-[300px] surface-raised`.

1. **충족도** — `[주간 | 오늘]` 스코프 토글 + 활동별 neutral 바 + `%` + `남음/달성/초과/—`.
   합계 행: `이번 주/오늘 기록 Xh · 목표 Yh`.
2. **최근 세션** — 오늘 기록 최대 8건. 활동 이름 + hue 점 + 시작시각 + 경과(라이브 `▶`).

---

## 9.7 녹화 진입점 매트릭스

녹화는 앱의 핵심 동작. **6개의 상호보완 진입점**(과거: 모달 1개 → 다원화):

| 진입점 | 위치 | 상태 | 동작 |
|---|---|---|---|
| **히어로 필** | 헤더 우측 | 항상 | 대기 `▶ 녹화`(indigo 필), 녹화중 `● 0:50`(red 필 + 펄스). 클릭=토글 |
| **NowCard CTA** | 사이드바 | 비활성 | `▶ 녹화 시작`(직전 활동 or 스위처) |
| **카드 호버 ▶** | 사이드바 활동 행 | 호버 | 해당 활동 즉시 시작 |
| **⌘⇧A 스위처** | 모달 | 단축키 | 필터 + ↑↓ + Enter로 활동 전환 |
| **⌘⇧R 글로벌** | OS 전역 | 단축키 | 토글(창 포커스 불필요). `shortcuts.rs` + `oxiline://quick-record` |
| **HUD** | 플로팅 패널 | ⌘⇧O | actionable 정지 버튼 |

히어로 필/NowCard/카드 호버는 `lastActivityId`로 resume; 없으면 스위처 오픈.

---

## 9.8 인터랙션 명세 (포인터 + 키보드)

| 대상 | 동작 | 입력 |
|---|---|---|
| 헤더 날짜 | 이전/다음일 | `‹` `›` / `←` `→` |
| 헤더 날짜 | 오늘 복귀 | `T` |
| 헤더 날짜 | 월 팝오버 | 마스트헤드 클릭 |
| 요일 칩 | 일 이동 | 칩 클릭 |
| 산화 바 | 시각 스크롤 | 바 클릭 |
| 타임라인 빈 공간 | 초안 생성 | 클릭 / 드래그 |
| PlanCard 본문 | 이동 | 드래그 |
| PlanCard 하단 | 크기 조절 | 핸들 드래그 |
| PlanCard | 삭제 | 호버 `×` |
| 사이드바 활동 | 계획 배치 | 타임라인으로 드래그 |
| DraftBlock | 커밋/취소 | `Enter` / `Esc` |
| 글로벌 | 커맨드 팔레트 | `⌘K` |
| 글로벌 | 환경설정 | `⌘,` |

---

## 9.9 단축키 체계

| 단축키 | 동작 | 범위 | 설정 키 |
|---|---|---|---|
| `⌘⇧O` | HUD 표시 | 전역 | `global_hotkey` |
| `⌘⇧R` | 퀵 토글 녹화 | 전역 | `quick_record_hotkey` |
| `⌘⇧A` | 활동 스위처 | 앱 내 | — |
| `⌘K` | 커맨드 팔레트 / 빠른 추가 | 앱 내 | — |
| `⌘N` | 오늘 빠른 추가 | 앱 내 | — |
| `⌘,` | 환경설정 | 앱 내 | — |
| `T` / `←` `→` | 오늘 / 전후일 | Day | — |
| `Enter`(PlanCard) | (예약) 블록에서 녹화 토글 | 타임라인 | 미구현 |

> `global_hotkey`/`quick_record_hotkey` 변경은 다음 실행 시 재등록(Preferences에서 안내).
> 라이브 재등록은 future nicety.

---

## 9.10 디자인 토큰 매핑 (요약)

전체 정의는 `src/tokens/*`. 본 명세가 사용하는 핵심 매핑:

| 용도 | 토큰 / 클래스 |
|---|---|
| Pane 3-tier | `surface-sunken` / `surface` / `surface-raised` |
| 선택(히어로/칩/오늘) | `interactive-primary` + `--shadow-today-node` |
| 입력 테두리 | `--input-shadow` / `--input-shadow-focus` (box-shadow, CSS border 아님) |
| 카드 그림자 | `--shadow-block-hover` / `--shadow-lg`(드래그) |
| 시간/숫자 폰트 | `font-mono tabular-nums` (Geist Mono) |
| 표시 폰트(날짜 등) | `font-display`/bold tabular (SUITE) |
| 동작 | `--duration-fast/base`, `--ease-out` |

---

## 9.11 검증 체크리스트 (현재 상태 기준, 전부 충족)

- [x] 3 pane이 surface-sunken / surface / surface-raised로 구분(라이트·다크).
- [x] 헤더가 단일 행(날짜·요일 칩·녹화 히어로) + 산화 바 스트립.
- [x] 산화 바 클릭 시 해당 시각으로 타임라인 스크롤.
- [x] 요일 칩이 마이크로 산화 바로 그 날 기록을 표현.
- [x] 타임라인 빈 공간 클릭/드래그 → 인라인 DraftBlock → Enter로 계획 생성.
- [x] PlanCard 본문 드래그로 이동(`update_plan` start 보존), 하단 핸들로 크기 조절, 호버 ×로 삭제.
- [x] 사이드바 ＋ 버튼/CTA로 활동 생성; 드래그로 타임라인 배치(드롭 피드백).
- [x] 헤더 히어로 필 한 번 클릭으로 녹화 시작/멈춤; ⌘⇧R 전역 토글.
- [x] 날짜 클릭 시 캘린더 팝오버가 body 포털로 확실히 뜸(drag-region 간섭 없음).
- [x] 녹화 중 트랜스포트/NowCard에 라이브 타이머; now-line이 양 레인 관통.
- [x] 실제 레인·최근 세션이 활동 이름(비 id) 표시.

---

## 9.12 구현 증거 (컴포넌트/함수 → 파일)

| 명세 항목 | 구현 위치 |
|---|---|
| 헤더 1행 + 산화 바 | `Header.tsx` (`Header`) |
| 요일 칩 + 마이크로 바 | `Header.tsx` (`DayChip`) |
| 캘린더 팝오버(body 포털) | `Header.tsx` (`createPortal`, `anchorRef`/`popBoxRef`) |
| 산화 바 컴포넌트 | `OxideBar.tsx` (`height`/`compact` prop) |
| 타임라인 모드/레인/now-line | `RecordTimeline.tsx` (`RecordTimeline`, `NowLine`) |
| 계획 생성(클릭/드래그/초안) | `RecordTimeline.tsx` (`PlanLane`, `DraftBlock`) |
| PlanCard 이동/리사이즈/삭제 | `RecordTimeline.tsx` (`PlanCard`) |
| 실제 레인(이름 표시) | `RecordTimeline.tsx` (`ActualLane`) |
| 사이드바 NowCard/라이브러리 | `Sidebar.tsx` (`NowCard`, `ActivityLibrary`, `DraggableActivity`) |
| 인스펙터 충족도/최근 세션 | `Inspector.tsx` (`ComplianceOverview`, `RecentSessions`) |
| DnD(드롭 분/스냅/OR 머지) | `lib/dnd.tsx` (`DndProvider`, `computeDropMinute`, `snapMinute`) |
| 이동/삭제/크기조절/생성 훅 | `hooks.ts` (`useMovePlan`, `useDeletePlan`, `useResizePlan`, `useCreatePlan`) |
| 글로벌 단축키 | `src-tauri/src/shortcuts.rs` (`register_quick_record`) |
| PlanSlot.weekday_mask | `oxiline-core/src/model.rs` + `plan.rs` + `record.rs` |
| 토큰 | `src/tokens/{primitives,semantic,semantic-dark,components,theme}.css` |

---

## 9.13 부록 — 진단 이력 (요약)

본 명세 이전, 2026-08-05 초에 보고된 6건 사용자 불만과 원인(전부 본 명세로 해소):

| # | 불만 | 원인 → 해소 |
|---|---|---|
| P1 | "구분감이 없다" | 3 pane 동일 배경 → 3-tier 표면 위계(§9.2) |
| P2 | "계획을 클릭/드래그로 생성했었는데" | 타임라인 표면 제스처 부재 → 클릭/드래그 생성 + 인라인 에디터(§9.4.2–3) |
| P3 | "사이드바에서 활동을 미리 만들어야" | 생성 진입점 부재 → ＋ 인라인 생성 + CTA(§9.5) |
| P4 | "녹화 시작을 단축키/버튼으로 못 한다" | 모달 1개 → 6 진입점 매트릭스(§9.7) |
| P5 | "날짜를 클릭해도 안 된다" | drag-region 중첩 → body 포털 팝오버(§9.3.3) |
| P6 | "직관적이지 않다" | 읽기 전용 표면 → 표면이 곧 인터페이스(§9.1) |

추가로 사용자 후속 요청으로 **이동/삭제**(§9.4.2)와 **헤더 1행 압축 + 인라인 에디터 정제**(§9.3, §9.4.3)를 반영했다.
