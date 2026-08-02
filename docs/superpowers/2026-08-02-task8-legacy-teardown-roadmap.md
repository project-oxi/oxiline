# Task 8 — 레거시 전면 철거 (task/routine/timeline → plan/record/slot 패러다임 전환)

> 날짜: 2026-08-02
> 의사결정: **전면 전환(C)** — CommandPalette/Header/BlockView를 recording-네이티브로 옮긴 뒤 legacy 코어·뷰·NowContext·테이블을 전부 제거.
> 5개 서브프로젝트로 분해(의존 순서). 각 서브프로젝트는 독립적으로 검증·커밋.

## 핵심 발견 (분해 전제)
레거시 코어는 dead code가 아니라 live 기능과 얽혀 있었다:
- `timeline.rs`: `get_now_context`(legacy) + `get_timeline_for_date/range`(**HUD OxideBar + Header 마커 + CommandPalette + WeekView** 사용).
- `tasks.rs`/`cards.rs`: **CommandPalette ⌘K**(task quick-add + card 제안) + BacklogView + BlockView.
- `routines.rs`: RoutineManager + now_context(notifier/tray).
- `reports.rs`: ReportView만.
- `NowContext`: 프론트는 이미 dead(HUD 마이그레이션 후), 백엔드는 notifier/tray/CLI.
- `BlockView`: **이미 dead code**(importer 없음).
- 색 시스템 2종: category `color_hue: number`(`categoryColor`) vs activity `hue_label: string`(`hueVar`). records는 activity hue_label → hueVar.

따라서 철거 = 패러다임 마이그레이션(의존 역순 제거).

---

## Sub1 — NowContext 제거 ✅ (`523ace2`)
- legacy `timeline::get_now_context` → recording-네이티브 `plan::now_summary`(활성 녹화 우선, else 현재 미해결 슬롯; next=다음 미해결 슬롯). 신모델 `NowSummary`/`NowEntry`(legacy cruft 제거).
- notifier/tray/CLI `now` 마이그레이션; dead `get_now_context` 명령 + 프론트 `useNowContext`/`onNowUpdate`/타입 제거.
- 검증: 코어 테스트 4(now_summary) + 워크스페이스 전부 green.

## Sub2 — CommandPalette ⌘K 하이브리드 재설계 ✅ (`41116af`)
- 활동 선택+Enter=녹화 시작; `@HH:MM`=그 시간 plan 생성; free-text=활동 생성 후 녹화/예약; 빈+Enter(녹화 중)=정지.
- task/card/backlog/timeline 의존 제거(useBacklog/useCreateTask/useSetTaskDone/useSuggestCards/useTimeline → useActivities/useStartRecord/useStopRecord/useCreatePlan/useCreateActivity/useRecordState). `useCreateActivity` 훅 추가.
- 검증: tsc + vitest 22 + vite build green. (인터랙티브 브라우저/.app smoke 권장 — followup #2.)

## Sub3 — Header 마커 + OxideBar timeline→records ✅ (`c2671db`)
- 주간 스트립 + 월 popover 마커(Header)와 일 미니맵(HUD OxideBar)을 records(`useRecordsRange`/`useDayRecords` + activity hue_label→`hueVar`)에서 도출. `useTimeline`/`qk.timeline` 제거.
- 공유 `lib/record-time.ts`(`isoLocal`)가 RecordTimeline/Inspector의 private UTC→local 복제를 대체.
- BlockView 삭제(dead code). `useSetSetting`의 `qk.timeline` 무효화 제거.
- 검증: tsc + vitest 22 + vite build green.
- 잔여: `useTimelineRange`(get_timeline)은 WeekView만 사용(sub4에서 제거).

## Sub4 — 레거시 4뷰 제거 (남음)
- **App.tsx**: BacklogView/WeekView/ReportView/RoutineManager import·렌더 제거; 뷰 전환 단축키 2/3/4 제거; `<RoutineManager/>` 제거; Escape의 `setRoutineManagerOpen` 제거. `view`/`setView` 미사용 → main은 항상 `<RecordTimeline/>`.
- **Header.tsx**: tabs(today/week/backlog/report) 배열+렌더 제거; RoutineManager 버튼(Layers 아이콘) 제거; `view`/`setView`/`setRoutineManagerOpen` destructure 제거; popover 날짜 선택의 `setView("today")` 제거(단일 뷰이므로); `Layers` import 제거.
- **store.ts**: `View` 타입 + `view`/`setView` + `routineManagerOpen`/`setRoutineManagerOpen` 제거(소비자 0 확인: App Escape·Header 버튼·RoutineManager 컴포넌트 전부 제거 대상).
- **dnd.tsx**: `data.kind === "backlog"`/`"block"` 분기 + `useUpdateTask` 제거(BacklogView/BlockView 사라져 dead; task 코어는 sub5).
- **삭제 파일**: BacklogView.tsx, WeekView.tsx, ReportView.tsx, RoutineManager.tsx.
- **i18n**: nav.week/backlog/report + backlog.*/routine.* 키 정리(옵션).

## Sub5 — 레거시 코어 삭제 + V5 마이그레이션 (남음)
- **코어 삭제**: `tasks.rs`, `routines.rs`, `reports.rs`, `cards.rs`, `timeline.rs`(전체) + 대응 테스트(activity/cards/reports/timeline 통합 테스트).
- **명령 삭제**(commands.rs + lib.rs): list_backlog/create_task/update_task/set_task_done/set_task_skipped/delete_task/materialize_if_virtual, list_routines/create_routine/update_routine/set_routine_active/delete_routine + routine_groups 일군, get_timeline/get_timeline_range, get_week_report/get_routine_streaks, suggest_cards.
- **프론트 hooks/api/types**: useBacklog/useCreateTask/useSetTaskDone/useSetTaskSkipped/useDeleteTask/useUpdateTask, useRoutines/useCreateRoutine/useUpdateRoutine/useSetRoutineActive/useDeleteRoutine + routine group 훅, useTimelineRange/useWeekReport/useSuggestCards + api·타입(Task/RoutineBlock/RoutineStreak/WeekReport/TimelineItem/CardSuggestion/CategoryBreakdown). ActivityRecord는 유간.
- **CLI**: task/routine/today/group 하위명령 + output 렌더러(task_text/now_text 외 legacy) 제거.
- **마이그레이션**: `V5__drop_legacy.sql`로 `tasks`/`routine_blocks`/`routine_groups`(및 관련) 테이블 DROP. (records/activities/plans/plan_options/compliance는 유지.)
- **notifier/tray**: sub1에서 recording-네이티브化 완료 — 영향 없음.
- 검증: `cargo test`(워크스페이스) + `cargo build` + `tsc`/`vite` 전부 green; 잔여 legacy 심볼 grep 0.

---

## 상태 (2026-08-02)
- ✅ Sub1, Sub2, Sub3 완료·검증·커밋.
- ⏳ Sub4, Sub5 남음(sub4 설계 완료, sub5 범위 확정).
- main: origin/main 대비 선행(푸시는 사용자 결정).
