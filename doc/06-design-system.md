# 06. Design System — OKLCH 토큰, 타이포그래피, 시그니처 비주얼

## 6.1 디자인 방향 선언

OxiLine은 "예쁜 캘린더 앱"들이 이미 점유한 자리 — 크림색 배경 + 파스텔 톤(Amie류) — 를 반복하지 않는다.
Oxi 생태계의 이름 자체가 **산화(oxidation)**에서 왔다는 점을 정체성의 근거로 삼는다. 산화는 두 얼굴을
가진다: 철이 녹스는 따뜻한 주황-적색(rust)과, 구리가 청록으로 변하는 차가운 녹청(patina/verdigris).
대부분의 사람은 "oxi"라는 이름을 들으면 반사적으로 rust의 주황색을 떠올린다 — 그래서 OxiLine은
**의도적으로 그 반대편, 녹청(verdigris)을 브랜드의 얼굴로 세운다.** 주황-적색은 완전히 버리지 않고,
"지연/경고"라는 의미론적 역할로만 아껴 쓴다. 이 결정 하나로 흔한 테라코타/웜톤 계열 AI 생성 디자인의
클리셰에서 벗어나면서도, 이름의 서사와 정확히 맞아떨어지는 팔레트가 만들어진다.

레이아웃의 시그니처는 **"산화 바(Oxide Bar)"** — 하루 전체를 가로 한 줄로 압축한 미니 진행 바다.
이는 사용자가 표현한 "재생 바처럼 흐른다"는 심상을 가장 직접적으로 구현하는 요소이자, 세로 타임라인
중심인 Structured/Sunsama/Amie 어디에도 없는 OxiLine 고유의 시각 언어다.

## 6.2 컬러 토큰 (OKLCH)

모든 색은 CSS 커스텀 프로퍼티로 `:root` / `[data-theme="dark"]`에 정의하고, Tailwind v4의 네이티브
OKLCH 지원을 통해 유틸리티 클래스에서 참조한다. 아래 값은 구현 시작점이며, 최종 명도 대비는 WCAG 2.x
기준 본문 텍스트 ≥4.5:1, 큰 텍스트/UI 요소 ≥3:1을 실측 후 미세 조정한다.

### 라이트 모드

```css
:root {
  /* Surface */
  --surface-canvas:   oklch(0.98 0.004 250);
  --surface-raised:   oklch(1.00 0    0);
  --surface-sunken:   oklch(0.95 0.006 250);
  --border-subtle:    oklch(0.88 0.008 250);
  --border-default:   oklch(0.80 0.010 250);

  /* Text */
  --text-primary:     oklch(0.22 0.015 250);
  --text-secondary:   oklch(0.48 0.014 250);
  --text-tertiary:    oklch(0.62 0.012 250);

  /* Brand — Oxide (verdigris) */
  --accent-oxide:        oklch(0.62 0.10 189);
  --accent-oxide-strong: oklch(0.52 0.12 189);
  --accent-oxide-subtle: oklch(0.94 0.03 189);

  /* Semantic — Rust (경고/지연 전용, 브랜드 색으로 쓰지 않음) */
  --signal-rust:        oklch(0.60 0.16 35);
  --signal-rust-subtle: oklch(0.94 0.04 35);

  /* Semantic — 완료/성공 */
  --signal-success:        oklch(0.65 0.14 145);
  --signal-success-subtle: oklch(0.94 0.03 145);

  /* Semantic — 정보 */
  --signal-info: oklch(0.60 0.09 240);
}
```

### 다크 모드

OKLCH의 강점은 다크 모드 변환이 "명도만 올리고 색상은 유지"하는 규칙적인 계산이 된다는 점이다
(§6.1에서 조사한 "luminance-first" 접근). 아래 값은 라이트 모드 대비 L을 올리고 필요한 만큼 C를
보정한 것이다.

```css
[data-theme="dark"] {
  --surface-canvas:   oklch(0.19 0.012 250);
  --surface-raised:   oklch(0.24 0.014 250);
  --surface-sunken:   oklch(0.15 0.010 250);
  --border-subtle:    oklch(0.32 0.014 250);
  --border-default:   oklch(0.40 0.016 250);

  --text-primary:     oklch(0.95 0.006 250);
  --text-secondary:   oklch(0.75 0.012 250);
  --text-tertiary:    oklch(0.58 0.014 250);

  --accent-oxide:        oklch(0.75 0.11 189);
  --accent-oxide-strong: oklch(0.82 0.13 189);
  --accent-oxide-subtle: oklch(0.30 0.05 189);

  --signal-rust:        oklch(0.72 0.15 35);
  --signal-rust-subtle: oklch(0.32 0.06 35);

  --signal-success:        oklch(0.75 0.13 145);
  --signal-success-subtle: oklch(0.30 0.05 145);

  --signal-info: oklch(0.72 0.10 240);
}
```

### 카테고리 팔레트 (사용자 지정 태그 색)

시맨틱 색(`accent-oxide`의 H=189, `signal-rust`의 H=35)과 겹치지 않는 6개 hue를 고정 L/C로 고정해
`categories.color_hue`에서 참조한다 (라이트: L=0.62 C=0.09 / 다크: L=0.74 C=0.11, hue만 가변).

| 카테고리 | Hue | 라이트 예시 | 다크 예시 |
|---|---|---|---|
| 업무 | 250 (인디고) | `oklch(0.62 0.09 250)` | `oklch(0.74 0.11 250)` |
| 건강 | 145 (모스그린) | `oklch(0.62 0.09 145)` | `oklch(0.74 0.11 145)` |
| 학습 | 300 (바이올렛) | `oklch(0.62 0.09 300)` | `oklch(0.74 0.11 300)` |
| 휴식 | 350 (로즈) | `oklch(0.62 0.09 350)` | `oklch(0.74 0.11 350)` |
| 개인 | 90 (올리브골드) | `oklch(0.62 0.09 90)` | `oklch(0.74 0.11 90)` |
| 기타 | — (무채색) | `oklch(0.62 0 0)` | `oklch(0.74 0 0)` |

## 6.3 타이포그래피

두 개 서체를 역할로 명확히 분리한다 — 이는 Oxi 생태계가 "CLI/터미널 태생"이라는 사실과 "한국어/영어
이중 언어 지원"이라는 요구를 동시에 만족시키기 위한 선택이다.

- **UI/본문 — Pretendard Variable**: 한글 렌더링 품질이 뛰어나고 2024~2026년 한국 프로덕트/개발자
  커뮤니티에서 사실상 표준으로 자리잡은 가변 폰트. 라틴 문자 폴백도 자연스러워 영문 UI에서도 이질감이
  없다. Variable weight axis(45~920)를 활용해 400/500/600/700 네 단계만 실사용.
  - 폴백 스택: `"Pretendard Variable", Pretendard, -apple-system, BlinkMacSystemFont, sans-serif`
- **시각/숫자/시간 — JetBrains Mono Variable**: 시계, 타임스탬프, 소요 시간, 그리고 CLI 출력과의
  시각적 연결고리로서 모든 "시간을 나타내는 숫자"에 고정폭 모노스페이스를 쓴다. 이는 장식이 아니라
  "이 앱은 시간을 재는 도구다"라는 메시지를 타이포그래피로 전달하는 장치다.
  - 폴백 스택: `"JetBrains Mono Variable", "JetBrains Mono", ui-monospace, SFMono-Regular, monospace`

### 타입 스케일

| 토큰 | 크기/행간 | 굵기 | 서체 | 용도 |
|---|---|---|---|---|
| `--type-display` | 28px / 34px | 700 | Pretendard | 창 헤더, 온보딩 |
| `--type-title` | 18px / 24px | 600 | Pretendard | 섹션 타이틀, HUD 현재 할일명 |
| `--type-body` | 14px / 20px | 400/500 | Pretendard | 할일 제목, 본문 |
| `--type-caption` | 12px / 16px | 500 | Pretendard | 보조 텍스트, 카테고리 라벨 |
| `--type-time-lg` | 32px / 32px | 500 | JetBrains Mono | HUD의 남은 시간, 큰 시계 |
| `--type-time-md` | 15px / 20px | 500 | JetBrains Mono | 타임라인 좌측 시각 눈금 |
| `--type-time-sm` | 11px / 14px | 500 | JetBrains Mono | 블록 내부의 소요시간 뱃지 |

## 6.4 형태 토큰

- 라운드: `--radius-sm: 6px`(뱃지), `--radius-md: 10px`(할일 블록), `--radius-lg: 16px`(카드, HUD
  패널), `--radius-full: 999px`(지금선 도트, 아바타형 요소).
- 간격: 4px 기반 스케일(`4/8/12/16/24/32/48`).
- 테두리: 기본은 그림자보다 **1px 헤어라인 보더**를 우선한다(`--border-subtle`). 그림자는 "떠 있는"
  요소(HUD 패널, 팝오버, 커맨드 팔레트)에만 제한적으로 사용해 "이건 오버레이다"라는 정보를 그림자
  자체가 전달하게 한다.
  ```css
  --elevation-panel: 0 8px 24px oklch(0.20 0.02 250 / 0.24), 0 2px 6px oklch(0.20 0.02 250 / 0.16);
  ```
- 아이콘: **Lucide** 아이콘 세트, 기본 스트로크 1.5px, 크기 토큰 `16/20/24px`.

## 6.5 시그니처 비주얼 #1 — 지금 선 (Now Line)

메인 타임라인(세로)을 가로지르는 얇은 수평선. `--accent-oxide-strong` 색, 두께 2px, 왼쪽 끝에
지름 10px의 발광 도트(`--accent-oxide` 배경 + `box-shadow: 0 0 0 4px var(--accent-oxide-subtle)`)와
그 옆에 `--type-time-sm` 모노스페이스로 실시간 `HH:MM`을 표시한다.

**움직임**: `requestAnimationFrame`으로 실제 초 단위까지 반영해 부드럽게 아래로 흐른다(1분마다
"뚝뚝" 끊기지 않는다 — 사용자가 명시적으로 요구한 "재생바처럼 계속 흘러가는" 느낌의 핵심 구현
포인트). 도트에는 2초 주기의 은은한 pulse(`opacity 0.85→1→0.85`, `scale 1→1.06→1`)를 줘서 "살아있음"을
표현하되, 과하지 않게 — `prefers-reduced-motion: reduce`이면 pulse와 부드러운 이동 애니메이션을 모두
끄고 1초 간격의 즉시 이동으로 대체한다.

**오디제이션 스윕(oxidation sweep)**: 지금 선이 어떤 블록을 통과하는 순간, 그 블록은 약 600ms에 걸쳐
"산화"된다 — 채도(chroma)가 약 60% 감소하고, 라이트 모드에서는 명도가 살짝 낮아지며, 다크 모드에서는
살짝 밝아진다(마치 색이 바래는 것처럼). 이는 장식이 아니라 "지나간 시간"을 시각적으로 되돌릴 수 없는
것처럼 표현하는 유일한 오케스트레이션 모먼트다 — 다른 모든 트랜지션(호버, 모달 열림 등)은 이보다
훨씬 절제된 120~200ms의 짧은 이징으로 처리해 이 순간이 상대적으로 더 특별하게 느껴지게 한다.

미완료 상태로 종료 시각을 10분 이상 넘긴 블록은, 산화 트랜지션이 끝난 뒤 왼쪽 보더가
`--signal-rust`로 은은하게 물든다(전체를 빨갛게 칠하지 않고 4px 좌측 보더만) — "이거 아직 안 했어요"를
공격적이지 않게 알려준다.

## 6.6 시그니처 비주얼 #2 — 산화 바 (Oxide Bar)

하루(설정된 `day_start_hour`~`day_end_hour`) 전체를 폭 100%의 얇은 가로 바 하나로 압축한 미니맵.
세 곳에서 동일한 컴포넌트로 재사용된다.

1. **메인 창 상단**: 타임라인 위에 고정된 얇은 스트립(높이 28px). 각 할일/루틴 블록이 비례 폭의
   색칠된 세그먼트로 표시되고, 현재 시각 위치에 도트가 있다. 클릭하면 해당 시간대로 세로 타임라인이
   스크롤된다 — 미니맵이자 내비게이션이다.
2. **플로팅 HUD**: HUD 패널 안에서 폭 320px 정도의 축소판으로 등장해, "오늘 전체에서 지금이 어디쯤
   있는지"를 텍스트 없이도 한눈에 전달한다.
3. **(Phase 2) 트레이 아이콘**: 메뉴바 아이콘 자체를 하루 진행률이 채워지는 얇은 바 형태로 그려,
   앱을 열지 않고도 곁눈질만으로 "하루가 얼마나 지났는지"를 인지하게 한다 (Raycast/CleanShot류
   트렌드 반영, `02-ux-research.md` §2.5).

바의 시각 규칙: 배경은 `--surface-sunken`, 각 세그먼트는 해당 카테고리 색의 저채도 버전(HUD처럼
작은 공간에서는 색이 서로 번지지 않도록 채도를 20% 낮춤), 지금 선 도트는 §6.5와 동일 스타일을
축소해 적용한다.

## 6.7 모션 토큰

```css
--motion-fast:  120ms;
--motion-base:  200ms;
--motion-slow:  320ms;
--motion-sweep: 600ms;   /* 오디제이션 스윕 전용 */
--ease-standard: cubic-bezier(0.2, 0, 0, 1);
--ease-emphasized: cubic-bezier(0.3, 0, 0.1, 1);
```

- HUD 패널 등장/소멸: `--motion-base` + `--ease-emphasized`, `opacity`와 `scale(0.96→1)` 동시 적용.
- 모달/커맨드 팔레트: `--motion-fast`, opacity만 (스케일 없음 — 너무 잦은 상호작용에는 스케일 애니메이션을
  아낀다).
- 리스트 아이템 완료 체크: 체크마크 draw-in `--motion-fast`, 이후 텍스트에 취소선이 `--motion-base`에
  걸쳐 그려짐.

## 6.8 다크/라이트 전환 및 i18n 연동 참고

- 테마는 `settings.theme`(`light`/`dark`/`system`)을 따르고, `system`일 때는
  `window.matchMedia('(prefers-color-scheme: dark)')`를 구독한다. 전환 시 색상 트랜지션은
  `--motion-base` 크로스페이드로 부드럽게 처리(급격한 깜빡임 방지).
- `react-i18next`의 언어 리소스는 `/src/locales/ko.json`, `/src/locales/en.json`으로 분리한다.
  Pretendard Variable은 한/영 모두 커버하므로 언어 전환 시 폰트 스위칭이 필요 없다 — 이 역시
  Pretendard를 선택한 실질적 이유 중 하나다.
- 날짜/시간 포맷은 로케일에 따라 분기한다: 한국어는 `2026년 7월 30일 (목)` 형태, 영어는
  `Thu, Jul 30` 형태. 시간 표기는 두 로케일 모두 24시간제 고정(§6.3에서 모노스페이스로 다루는 시간
  숫자의 일관성을 위해 — 12시간제 AM/PM 배지를 별도로 넣지 않는다).

## 6.9 접근성 품질 기준선

- 모든 인터랙티브 요소에 가시적 포커스 링(`outline: 2px solid var(--accent-oxide); outline-offset: 2px`).
- 색만으로 정보를 전달하지 않는다 — 카테고리는 색 + 아이콘, 지연 상태는 색 + 좌측 보더 두께 변화로
  이중 인코딩.
- `prefers-reduced-motion` 존중(§6.5).
- 키보드만으로 전체 앱 조작 가능(`07-ui-screens-and-flows.md`의 단축키 표 참고).
