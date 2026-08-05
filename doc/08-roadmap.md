# 08. Roadmap — 구현 단계와 완료 기준

그린필드 원샷 구현 시, 아래 순서를 그대로 우선순위로 삼는다. Phase 0~1이 사실상 "MVP"이고, 이 문서
세트로 만들 1차 구현의 목표 범위다. Phase 2/3는 이후 반복 작업을 위한 참고용으로 남겨둔다.

## Phase 0 — 프로젝트 골격

- [ ] Cargo workspace 생성 (`oxiline-core`, `oxiline-app`, `oxiline-cli`) — `04-architecture.md` §4.2
- [ ] `oxiline-core`: SQLite 스키마 마이그레이션 파일 작성 및 `rusqlite_migration` 연결 — `03-data-model.md`
- [ ] `oxiline-core`: 도메인 타입(`RoutineBlock`, `Task`, `Category`, `TimelineItem` 등) + `specta::Type` 파생
- [ ] `oxiline-core`: `get_timeline_for_date`, `materialize_occurrence`, CRUD 함수 유닛 테스트
      (특히 요일 마스크 경계값, 한시적 루틴 기간, 구체화 중복 방지 유니크 인덱스 동작을 테스트)
- [ ] Tauri v2 + React 19 + Vite 스캐폴드, Tailwind v4 연결, OKLCH 토큰 CSS 파일 작성 — `06-design-system.md`
- [ ] `react-i18next` 초기 셋업, `ko.json` / `en.json` 뼈대(빈 값이라도 키 구조는 전체 확정) · CLI 출력은 손작성 ko/en 테이블

**완료 기준**: `cargo build --workspace`가 통과하고, `oxiline-cli`가 빈 DB에 대해 `oxiline doctor`를
정상 출력한다.

## Phase 1 — MVP

### 코어 기능
- [ ] Day Timeline 뷰 렌더링 (세로 시간축, 블록 절대 위치, 겹침 처리) — `07-ui-screens-and-flows.md` §7.1
- [ ] 지금 선(Now Line) 실시간 렌더링 + 오디제이션 스윕 — `06-design-system.md` §6.5
- [ ] 할일 CRUD (백로그 포함), 루틴 CRUD, 카테고리 CRUD — GUI + CLI 양쪽
- [ ] 가상 occurrence 병합 및 지연 구체화 동작 확인(완료/건너뛰기/시간수정 각각 실제로 `tasks` 행을
      만드는지)
- [ ] 백로그(Inbox) 뷰 — `07-ui-screens-and-flows.md` §7.2
- [ ] 루틴 관리 패널 — §7.4

### 백그라운드/전역 기능
- [ ] 트레이 아이콘 상주, `ActivationPolicy::Accessory`로 Dock 아이콘 숨김 — `04-architecture.md` §4.3
- [ ] 메인 창 닫기 → 숨기기로 가로채기, 트레이 메뉴의 "종료"만 실제 종료
- [ ] `tauri-plugin-autostart` 연결(기본 ON)
- [ ] `tauri-plugin-global-shortcut` + `tauri-nspanel` 기반 플로팅 HUD, 2초 자동 소멸 — §4.4, §7.6
- [ ] `tauri-plugin-single-instance` 연결

### CLI
- [ ] `oxiline now / today / task * / routine * / category * / settings * / doctor` 전체 구현,
      `--json` 및 종료 코드 스펙 준수 — `05-cli-spec.md`
- [ ] GUI ↔ CLI 동기화: `notify` 파일 감시 → `db-changed` 이벤트 → React Query invalidate — §4.5

### 디자인/설정
- [ ] 다크/라이트 테마 전환(시스템 연동 포함)
- [ ] 언어 전환(한/영), 로케일별 날짜 포맷
- [ ] 환경설정 패널 전체 섹션 — §7.8
- [ ] 온보딩 3단계 — §7.9
- [ ] 커맨드 팔레트(⌘K) 기본 캡처 + `@시간` 힌트 파서 — §7.5
- [ ] 키보드 단축키 표 전체 구현 — §7.10

**완료 기준(수용 기준)**:
1. 앱을 처음 실행하면 온보딩을 거쳐 오늘 날짜의 빈 타임라인이 보인다.
2. `oxiline routine add`로 CLI에서 루틴을 추가하면, GUI를 재시작하지 않아도 수 초 내로 메인 창
   타임라인에 나타난다.
3. 전역 단축키를 다른 앱(예: 전체화면 브라우저) 위에서 눌렀을 때 HUD가 뜨고, 포커스가 그 앱에
   그대로 유지된 채(타이핑 중이던 커서가 사라지지 않음) 2초 후 사라진다.
4. 메인 창을 닫아도 메뉴바 아이콘이 남아 있고, 그 상태에서도 전역 단축키가 동작한다.
5. `oxiline task done <id> --json` 실행 시 올바른 JSON과 종료 코드 0을 반환하고, 존재하지 않는
   id에는 종료 코드 3과 `not_found` 에러를 반환한다.
6. 다크/라이트, 한/영 전환이 재시작 없이 즉시 반영된다.

## Phase 2 — 다듬기 (MVP 이후)

- [ ] 산화 바(Oxide Bar) 컴포넌트 — 메인 창 헤더 + HUD 축소판 — `06-design-system.md` §6.6
- [ ] 주간(Week) 뷰 — `07-ui-screens-and-flows.md` §7.3
- [x] 드래그 앤 드롭으로 백로그 → 타임라인 스케줄링, 블록 리사이즈/이동
- [x] 워크로드 경고 톤 변화(빠듯함 상태) 실장
- [x] 트레이 아이콘에 실시간 진행률 그려 넣기(§6.6의 3번째 재사용처)
- [ ] 루틴 그룹(`routine_groups`) UI 노출 — 일괄 on/off
- [ ] 네이티브 알림(옵트인) — 블록 시작 시 macOS 알림
- [ ] 접근성 감사(키보드 전용 조작, 스크린리더 라벨 전수 점검)

## Phase 3 — 확장 아이디어 (평가 후 채택)

- [ ] `oxiline mcp serve` — MCP 서버 모드로 에이전트 연동 고도화 — `05-cli-spec.md` §5.6
- [ ] Apple Calendar(EventKit) 읽기 전용 임포트 — 다른 캘린더 이벤트를 타임라인에 참고용으로 겹쳐보기
      (동기화 아님, 로컬 읽기 전용 스냅샷)
- [ ] Shortcuts.app 연동(AppleScript/URL scheme)으로 시스템 단축어에서 할일 추가
- [ ] 습관 스트릭/주간 리포트 — Non-goal에서 재검토 대상이지만, 게이미피케이션이 아니라 "완료율을
      담백하게 보여주는" 수준으로 제한할지 재논의 필요
- [ ] 코드 서명/공증 포함 릴리즈 자동화(CI)

## 구현 시 우선순위 판단 원칙

문서 세트 전체에서 스펙이 상충하거나 모호해 보이는 지점을 만나면, 아래 순서로 판단한다.

1. `01-product-vision.md`의 Non-goals에 걸리는 기능은 아무리 "있으면 좋아 보여도" 만들지 않는다.
2. 세로 타임라인(핵심 정보 구조)과 산화 바(시그니처 장식)가 구현 리소스를 놓고 경쟁하면, **세로
   타임라인을 항상 먼저** 완성한다 — 산화 바는 Phase 2로 미뤄도 제품이 성립하지만, 세로 타임라인
   없이는 제품이 성립하지 않는다.
3. GUI 전용 기능(드래그 앤 드롭, 커맨드 팔레트 자연어 힌트 등)을 core crate로 새어 들어가게 하지
   않는다 — core는 항상 GUI/CLI 어느 쪽도 모르는 순수 라이브러리로 유지한다 (`04-architecture.md` §4.2).
