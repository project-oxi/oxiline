# OxiLine — 남은 할 일 (hud-date-orplan 후속)

> 기준: main `939f8d2` (2026-08-02). 3기능(HUD 재구성·날짜 popover·OR-계획 다중선택+병합+리사이즈)은 구현·리뷰·브라우저 smoke 완료 후 main 병합됨.
> 관련 문서: spec `docs/superpowers/specs/2026-08-02-hud-date-orplan-design.md`, plan `docs/superpowers/plans/2026-08-02-hud-date-orplan.md`.
> 본 문서는 **별도 세션에서 진행**하기 위한 후속 작업 목록.

## 완료된 것 (참고용)
10커밋(`bd566f3..939f8d2`):
- `4fd6750` 코어 `resize_plan` · `9fed8f5` 명령(`add_plan_option`/`resize_plan`)+TS 플러밍
- `b494efb` HUD 녹화-네이티브 재구성
- `96d75f5`+`12225c8` 날짜 popover 월 달력(월이동 drift 수정)
- `9655c7f`+`ec345b8` OR 다중선택+드롭-병합(드롭 후 선택 해제)
- `7867ff6`+`0e04382` 계획 카드 리사이즈 핸들(StrictMode/pointercancel 수정)
- `939f8d2` gitignored 스크래치 언트랙

검증: vitest 22/22(순수로직), 코어 27개, `cargo build`+`tsc`/`vite` clean, 브라우저 smoke로 드롭-병합/다중선택/리사이즈/popover 제스처 확인.

---

## 후속 할 일

### 1. 원격 푸시 (배포)
- **상황**: main가 origin/main보다 **62커밋 앞서** 있음(로컬 전용).
- **할 일**: `git push origin main` (또는 별도 PR 브랜치).
- **확인**: 원격에 반영 + CI(있으면) green.

### 2. 실제 .app 엔드투엔드 smoke (검증 갭)
- **상황**: 브라우저 목업(`tauri-v2-browser-audit-mock`)은 React/제스처 로직 + 목킹된 invoke 경로만 검증. **실제 Tauri 명령 경로(Rust↔JS 인자 변환, 진짜 DB 지속화)는 .app에서 미실행**.
- **할 일**: macOS .app 빌드(`oxiline-build-install` 스킬) 후 손체크:
  - ⌘⇧O HUD: 녹화 중 `● 활동·경과·주간 막대`, 자유 시간 `지금 예정/자유 시간` + `다음 …`
  - 활동 드래그→타임라인 빈 곳: 단일 계획 생성 / ⌘다중선택 드래그: OR 계획
  - 활동 드롭→기존 계획 카드: 옵션 추가(OR 증가)
  - 카드 하단 핸들 드래그: 5분 스냅 리사이즈 → DB 반영 확인(재시작 후 유지)
  - 날짜 제목 클릭: 월 달력 + 마커, 날짜 선택/외부클릭 닫힘, ‹› 월 이동(월 skip 없음)
- **왜**: arg-case 대응·코어 fn은 코드 리뷰+단위테스트로만 확인됨.

### 3. HUD show→리프레시 런타임 검증 (검증 갭)
- **상황**: HUD는 별도 영구 webview라 브라우저 smoke에 미포함. `oxiline://hud-show` 이벤트→`invalidateQueries`→refresh가 **코드 리뷰에만 의존**.
- **할 일**: #2 .app smoke에 포함 — ⌘⇧O를 여러 번 눌렀을 때 녹화 세션/슬롯 데이터가 최신으로 갱신되는지 확인(숨김 상태에서 stale 안 되는지).
- **왜**: 표시 시점 최신 데이터 보장이 HUD의 핵심 계약.

### 4. [기술부채] 다중선택 드롭-병합 sort_order 경쟁 (parked)
- **상황**: 다중 활동을 기존 카드에 드롭하면 dnd.tsx가 N개의 `add_plan_option`을 **개별 풀 커넥션**으로 발화. 코어 `plan::add_option`의 `MAX(sort_order)+1` 읽기+쓰기가 비원자(트랜잭션 아님)라 WAL에서 동시 커넥션이 같은 MAX를 읽어 sort_order 충돌 가능.
- **영향**: 표시 순서만 비결정적(데이터 손상 无, `busy_timeout`이 쓰기 직렬화, `→실행` 해석은 activity_id 사용). **머지 안막음**.
- **할 일**: 벌크 명령 `add_plan_options(plan_id, activity_ids: Vec<String>)` 신설 — 단일 트랜잭션에서 순차 sort_order 배정(`update_plan`의 옵션 재작성 패턴 참고). 코어 → commands.rs/lib.rs 등록 → api.ts/hooks.ts → dnd.tsx의 `forEach(addOption)`를 1회 호출로 교체.
- **파일**: `crates/oxiline-core/src/plan.rs`, `commands.rs`, `lib.rs`, `api.ts`, `hooks.ts`, `lib/dnd.tsx`(드롭-병합 분기).
- **확인**: 동시 다중 추가 시 sort_order가 단조증가/유일.

### 5. [큰 작업] Task 8 — 레거시 전면 철거
> ⚠️ **의존성 주의**: `timeline::get_now_context`가 아직 notifier/tray/CLI에 의존 중이라 `timeline.rs` 단순 삭제 불가. 선결 과제 있음.

현재 남은 레거시:
- **뷰**: `App.tsx`가 `BacklogView`/`WeekView`/`ReportView`/`RoutineManager` 임포트·렌더(L4-7, 91-93, 102).
- **코어 모듈**: `tasks.rs`·`routines.rs`·`reports.rs`·`cards.rs`·`timeline.rs`.
- **NowContext 경로**: `get_now_context` 명령(commands.rs:216) + `getNowContext`/`onNowUpdate`/`useNowContext`(api.ts/hooks.ts). **단, 실사용처**: `notifier.rs:49`, `tray.rs:120`, CLI `main.rs:60`, 그리고 프론트 `useNowContext` 사용처(Inspector 등 — 사용처 먼저 조사).

**진행 순서 제안**:
1. **선결**: notifier/tray/CLI의 "현재/다음" 알림을 recording 레이어(`useRecordState`/`PlanSlot` 기반)로 마이그레이션 → `get_now_context` 호출 제거.
2. 프론트 `useNowContext`/`onNowUpdate` 사용처 전수 조사 후 제거.
3. `App.tsx`에서 레거시 4뷰 제거(뷰 전환 탭 `backlog/week/report` + RoutineManager 트리거). recording-네이티브 대체 뷰가 들어간 뒤에만.
4. 코어 `tasks.rs/routines.rs/reports.rs/cards.rs/timeline.rs` + 대응 테스트 제거.
5. 별도 마이그레이션 `V5__drop_legacy.sql`로 `tasks`/`routine_blocks`(및 관련) 테이블 드롭.
- **왜 순서 중요**: 뷰/사용처가 살아있을 때 드롭하면 앱이 깨짐(메모리 메모 참고).

---

## 메모
- vitest는 **순수로직 전용**(node env, no DOM) — 컴포넌트/DnD 검증은 항상 브라우저 목업(#2) 또는 .app 손체크 필요.
- dnd-kit 중첩 droppable: 평범한 `rectIntersection`은 큰 컨테이너가 이김 → 커스텀 collisionDetection으로 안쪽 droppable 선호(`lib/dnd.tsx` `nestedCollision` 참고).
