# OxiLine UI 리디자인 — 설계 사양 (Design Spec)

> **날짜:** 2026-07-31
> **스코프:** DayTimeline, BlockView, NowLine, Header, styles.css
> **프로세스:** brainstorming (사용자가 대화형 게이트를 면제 — `auto-task` "사용자에게 절대 질문하지 말 것")
> **설계자:** agent (최고 디자이너 마인드)

---

## 0. 핵심 발견 — 코드 재검증 결과

원본 태스크 문서의 "현재 상태 / 문제점" 단락은 **구 버전을 기준**으로 작성되었다.
실제 코드(`crates/oxiline-app/src/`)를 검증한 결과, 요구사항 대부분이 **이미 구현되어 있다**:

| 태스크 문서의 가정 | 실제 코드 상태 | 결론 |
|---|---|---|
| "시간 구분 가로선(시/30분)이 촌스럽게 두껍다" | `DayTimeline.tsx:150` 주석 `{/* quiet time labels — no gridlines */}`. **그리드라인이 아예 없다.** 시간은 mono 텍스트 라벨만. | ✅ 이미 해결 |
| "30분 표시선 제거 또는 더 얇게" | 30분선 자체가 존재하지 않음 | ✅ 이미 해결 |
| styles.css: "디자인 토큰 재정의(OKLCH), 다크/라이트, 스크롤바" | 이미 oxi-design-system v1.0 **3-티어 토큰**(primitive → semantic → component), APCA 최적화 색, `.dark` 단일 트리거, thin 커스텀 스크롤바(`styles.css:311-324`), `prefers-reduced-motion` 지원 | ✅ 이미 해결 |
| NowLine: "너무 튀지 않게" | 이미 1.5px gradient-to-transparent + 22% halo pulse dot. 미묘함 | ✅ 이미 해결 |
| `06-design-system.md` 토큰(Pretendard/JetBrains/verdigris) | 해당 문서는 **PARTIALLY SUPERSEDED**(line 2-13). styles.css는 이미 SUIT/SUITE/Geist Mono + 6-hue 팔레트로 이관 완료 | ⚠️ 회귀 금지 |

**따라서 본 리디자인의 목표는 "고쳐야 할 결함"이 아니라, 이미 훌륭한 기반 위에서
*진정한 프리미엄-미니멀* 마감작업을 하는 것이다.** 회귀(regression)를 절대 피한다.

---

## 1. 현 디자인의 진짜 갭 (디자이너 눈으로)

2025-2026 캘린더/태스크 앱 트렌드(Linear, Cron, Amie, Superlist, Sunsama)의 공통 언어:
**평평한 표면(flatten) + 헤어라인 + 색은 데이터로만 + hover/활성 상태에서만 elevation**.
반면 현재 OxiLine은:

### Gap A — 그림자 중복(busy elevation)
- `DayTimeline.tsx:128` 외부 컨테이너: `rounded-2xl` + `shadow-lg`
- `BlockView.tsx:140-144` 모든 블록: `shadow-sm` (rest) → `shadow-lg` (hover)
- 좁은 420px 창에서 "큰 카드 안의 작은 카드들이 각자 그림자" → 시각적 노이즈.
  프리미엄 앱들은 블록을 **flat tinted fill + 헤어라인**으로 두고 hover에서만 lift.

### Gap B — 카테고리 색이 거의 안 보인다 (scannability)
- `BlockView.tsx:92-94` 블록 배경: `color-mix(in oklch, accent 8%, surface-raised)` — 8% 틴트라
  거의 무채색. 색이 보이는 유일한 닻은 4×4px 체크박스 원. 한눈에 카테고리 분류가 안 됨.
- **해결:** 좌측 3px accent rail(카테고리 색) — "color is data" 정규 패턴. 그림자 의존도를
  낮추면서 분류 가시성을 얻는다.

### Gap C — 블록 elevation 단계가 거칠다
- rest(`shadow-sm`) → hover(`shadow-lg`) 점프가 크다. 중간 단계(`shadow-md`) lift가 없어
  둥둥 뜨는 느낌. `drag`는 `shadow-lg`로 hover와 동일 → 드래그가 특별하지 않음.

### Gap D — 헤더 상단 여백 부족 (트래픽라이트 압박)
- `Header.tsx:37` 컨테이너 `px-4 pb-2` — **상단 패딩이 없다**. 첫 행 `py-1.5`(6px).
  Overlay 타이틀바에서 콘텐츠가 창 최상단에 거의 붙어 시작 → 답답/잘림 느낌.
  (트래픽라이트는 `pl-[56px]`로 좌우 충돌은 피했으나 수직 여백이 빠듯.)

### Gap E — NowLine 라벨 가독성
- `NowLine.tsx:79-83` 시각 라벨이 **배경 없는 mono 텍스트**. 블록 위에 겹치면 텍스트가
  블록 제목과 충돌해 읽기 어렵다. 작은 pill 배경이 필요.

### 비-Gap (의도적으로 손대지 않음)
- spine + 카테고리 dot: 철도(railroad) 메타포의 시그니처. dot은 done/past 상태 인코딩.
  유지하되 spine 두께만 미세 조정.
- OxideBar(일 미니맵), week strip, segmented tabs, workload footer: 이미 깔끔. 구조 유지.
- 타이포그래피(SUIT/SUITE/Geist Mono), 컬러 토큰: 이미 정규 시스템. 회귀 금지.

---

## 2. 접근법 3가지 + 추천

### 접근법 1 — "Flat Premium" (추천 ⭐)
블록을 **flat tinted fill + 1px 헤어라인 + 좌측 accent rail**로 전환. 그림자는
hover(`shadow-md` lift)/drag(`shadow-lg`+scale)에서만. 외부 카드는 `shadow-lg`→`shadow-sm`.
헤더에 상단 여백. NowLine 라벨 pill. spine 2px→1px. 헤어라인 시간 눈금 tick(옵션).

- **장점:** 가장 트렌드에 부합. 색=데이터 가시성 ↑. 노이즈 ↓. 회귀 위험 최소(구조不变).
- **단점:** 블록이 "뜨지 않아" 평평해 보일 수 있으나 — 이게 의도.

### 접근법 2 — "Refined Elevated"
현재 그림자 접근을 유지하되 톤만 다듬음(외부 `shadow-md`, 블록 `shadow-xs`→`shadow-sm`).
accent rail만 추가.

- **장점:** 변경 최소.
- **단점:** Gap A(busy elevation)를 근본적으로 해결 못 함. "전면 리디자인" 기대 미달.

### 접근법 3 — "Glass / Frosted"
블록과 패널을 `backdrop-blur` 반투명으로. 2025 frosted-glass 트렌드.

- **장점:** 화려.
- **단점:** 불투명 `bg-surface` 바디 위에서 blur 효과가 거의 안 남(의미 없음).
  HUD에나 적합. 본 창에는 과함 + 성능 비용. **기각.**

### 결정: **접근법 1 (Flat Premium)**.

---

## 3. 상세 설계

### 3.1 styles.css — 타임라인 전용 토큰 레이어 추가 (회귀 없는 확장)

기존 3-티어 아키텍처(Tier 3 component tokens, `styles.css:138-184`)를 **확장**만 한다.
새 토큰들(Tier 3 component tier에 추가, `.dark` 패리티 유지):

```css
/* Timeline surface — outer card */
--tl-card-radius: var(--radius-2xl);     /* 유지 */
--shadow-card:   var(--shadow-sm);       /* shadow-lg → shadow-sm 로 감량 */

/* Block elevation ladder (세 단계: rest / hover / drag) */
--shadow-block-rest:  none;              /* flat — 헤어라인으로 분리 */
--shadow-block-hover: var(--shadow-md);
--shadow-block-drag:  var(--shadow-lg);

/* Block surface — tinted flat fill (surface-raised보다 한 톤 구분) */
--color-block-bg: oklch(96.5% 0.005 95);          /* light */
.dark { --color-block-bg: oklch(24% 0.016 265); } /* dark (surface-raised와 미세 차이) */

/* Block hairline border (rest 상태 분리용) */
--color-block-border: oklch(90% 0.007 95);
.dark { --color-block-border: oklch(33% 0.015 265); }

/* Category accent rail (좌측 3px 색 막대) */
--tl-rail-width: 3px;

/* Spine */
--tl-spine-width: 1px;                    /* 2px → 1px */

/* Hour tick notch (spine 상의 시간 눈금, 옵션) */
--tl-tick-color: oklch(0% 0 0 / 0.06);
.dark { --tl-tick-color: oklch(100% 0 0 / 0.05); }
```

`@theme inline`에 `--color-block-bg`, `--color-block-border` 노출(컴포넌트가 유틸리티로 소비).
나머지는 `var()` 직접 참조(컴포넌트 conditional 값용 — 정규 허용 범위, `styles.css:11`).

### 3.2 BlockView.tsx — flat card + accent rail

```
┌─│─────────────────────────┐   ← 좌측 3px accent rail (카테고리 색 / done시 success)
│ ○ 할 일 제목               │   ← 체크박스 원(유지) + 제목
│   09:00–10:00              │   ← (높을 때) 시간대
│   60분                     │   ← (더 높을 때) 소요시간
└────────────────── ✥ ──────┘   ← resize handle(유지)
```

- `style.background`: `accent 8% mix` → `var(--color-block-bg)` (flat tinted fill).
- **추가:** 좌측 rail 자식 `<div>` — `width: var(--tl-rail-width)`, `background: accent`
  (done → `--color-status-success`, past-undone → 유지 but 낮은 opacity). `inset-y` full height.
- 테두리: `border: 1px solid var(--color-block-border)` (rest) → hover 시 border-color 밝게.
- 그림자: rest `var(--shadow-block-rest)`(none) → hover `var(--shadow-block-hover)`(md)
  → drag `var(--shadow-block-drag)`(lg) + `scale(1.02)`.
- radius: `rounded-lg`(유지, `--radius-lg`).
- opacity 로직(done/past/virtual): 기존 유지.
- 체크박스·제목·시간·resize handle: 레이아웃 유지, 패딩 미세 조정(rail 공간 확보 `pl` 증가).

### 3.3 DayTimeline.tsx — flatten + 정밀 spine

- 외부 카드(`:128`): `boxShadow: var(--shadow-lg)` → `var(--shadow-card)`(sm). radius·bg 유지.
- spine(`:145-148`): `width: 2` → `var(--tl-spine-width)`(1px). 색 `var(--color-border)` 유지.
- **시간 tick notch 추가(미묘, 확정):** 각 hour 위치에서 spine 좌측에 6px·1px 높이 tick,
  `background: var(--tl-tick-color)`. 그리드라인 아님 — spine 위 작은 눈금만. 리듬감 부여.
- 시간 라벨·spine dot·OxideBar·footer·composer·rubber-band·overflow chip: **변경 없음**.

### 3.4 NowLine.tsx — 라벨 pill

- 라인·dot·pulse: **유지**(이후 미묘).
- 라벨(`:79-83`): 배경 pill 추가 — `background: var(--color-surface-raised)`,
  `box-shadow: var(--shadow-xs)`, `border-radius: var(--radius-xs)`, `padding: 1px 5px`.
  블록 위에서도 HH:MM 가독 확보. 글자색 `--color-interactive-primary` 유지.

### 3.5 Header.tsx — 상단 여백 + 미세 정제

- 컨테이너(`:37`): `px-4 pb-2` → `px-4 pb-2 pt-2` (상단 8px 여백 — 트래픽라이트 호흡).
- 첫 행(`:40`) `py-1.5` 유지(이제 pt-2가 전체 여백 담당).
- 날짜 타이틀·week strip·tabs: **레이아웃 유지**. tab 활성 상태 token 이미 `--shadow-sm`
  사용 중이라 정규 준수. 변경 최소.

---

## 4. 비교소 (Ground truth 비교)

- **Linear / Cron:** flat blocks, 좌측 색 rail, hover lift. ≈ 접근법 1.
- **Amie:** 파스텔 틴트 블록(우리는 8%→tinted flat, 더 절제).
- **Notion Calendar:** 헤어라인 + flat.
- 본 설계는 oxi "ink on paper, color is data, calm authority" 원칙과 정합.

## 5. 검증 기준 (Acceptance)

1. `bun run build`(Vite) 성공 — 타입/번들 에러 없음.
2. `cargo build`(Rust) 성공.
3. styles.css: `dark:` variant 컴포넌트 파일에 새로 들어가지 않음(토큰 레이어만).
4. 회귀 체크: 그리드라인 재도입 없음 / 폰트 토큰(SUIT/SUITE/Geist Mono) 유지 / 스크롤바·reduced-motion 유지.
5. 시각: 블록 flat + 좌측 색 rail 보임 / hover에서만 lift / 헤더 상단 여백 / NowLine 라벨 pill.
6. `cargo tauri build` → `OxiLine.app` 번들 생성.

## 6. 비목표 (Out of scope)

- ReportView / WeekView / BacklogView / CommandPalette / Preferences / RoutineManager 시각 변경.
- 데이터 모델·기능 변경 (resize, 다중 카드, 템플릿 등은 별도 태스크).
- HUD 창 디자인.
- 그리드라인 재도입 (사용자가 싫어함 — "none"이 정답).
