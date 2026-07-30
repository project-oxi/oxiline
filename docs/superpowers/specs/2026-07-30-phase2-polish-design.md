# Phase 2 다듬기 — 설계 스펙

> 그린필드 OxiLine의 Phase 2(다듬기) 구현을 위한 설계 문서. Phase 0/1(MVP)은 빌드·테스트·스모크 검증 완료된 상태에서, 본 문서는 그 이후 라운드를 정의한다.
>
> 본 라운드는 **테스트 작성 없이 구현만** 진행한다. 테스트는 모든 구현이 끝난 뒤 별도 라운드에서 일괄 추가한다 (`08-roadmap.md`의 "완료 기준"과 별개).

## 0. 의존 그래프와 작업 순서

Phase 2의 7개 항목은 다음 의존을 가진다. 이 순서대로 구현한다 (앞 항목이 뒤 항목의 전제).

```
[1] tauri-nspanel HUD          ← macOS 네이티브 1순위, Phase 1 인수 기준 3번 완전 충족
[2] tauri-plugin-notification   ← [1]과 독립, 의존성만
[3] 백그라운드 알림 스케줄러    ← [2] 의존
[4] 트레이 진행률 그리기         ← [1]과 독립, 산화 바 색상 토큰 재사용
[5] 주간(Week) 뷰               ← 타임라인 컴포넌트 재사용, [1~4]와 독립
[6] 드래그 앤 드롭              ← [5]의 `useTimeline` 훅 재사용, 가장 큰 단일 작업
[7] 루틴 그룹 CRUD + 인라인 편집 + 접근성 마무리 ← UI 종합, 마지막에
```

## 1. HUD tauri-nspanel 마이그레이션

**목적**: Phase 1 인수 기준 3번("전역 단축키가 다른 앱 위에서 눌렸을 때 HUD가 뜨고, 포커스가 그 앱에 그대로 유지된 채 2초 후 사라진다") 완전 충족. 현재는 transparent always-on-top 오버레이라 일부 환경(전체화면 앱 위)에서 포커스 theft 발생 가능.

**변경**:

- `crates/oxiline-app/src-tauri/Cargo.toml`: `tauri-nspanel = "2"` 추가. `cfg(target_os = "macos")`로 게이팅.
- `crates/oxiline-app/src-tauri/src/hud.rs`:
  - macOS: `tauri_nspanel::PanelBuilder`로 NSPanel 빌드. `no_activate(true)`, `level(PanelLevel::Floating)`, `style_mask(StyleMask::empty().hud_window())`, `corner_radius(16)`, `transparent(true)`.
  - 비-macOS: 기존 transparent always-on-top 오버레이 그대로 (CI/리눅스 빌드 대응).
  - `position_top_center()` 재사용.
- `crates/oxiline-app/src-tauri/src/lib.rs`:
  - `setup`에서 `hud::init_panel(app.handle())` 추가 (사전 빌드, 평소엔 숨김).
  - `on_window_event`는 hud 패널에는 적용하지 않음.
- `tauri.conf.json`: hud window 정의 그대로 유지 (`"visible": false`, `"focus": false`, `"decorations": false`, `"transparent": true`).

**검증**: macOS GUI 부팅 후 전체화면 브라우저 위에서 `⌘⇧O` → 텍스트 입력 커서 유지 확인 (사용자 수동 검증).

## 2. tauri-plugin-notification (설정 + 권한)

**목적**: macOS 알림 권한 요청 및 토글.

**변경**:

- `crates/oxiline-app/src-tauri/Cargo.toml`: `tauri-plugin-notification = "2"`, `tauri-plugin-opener = "2"` 추가. opener는 macOS 시스템 설정(`x-apple.systempreferences:...`)을 외부에서 열기 위함.
- `crates/oxiline-app/src-tauri/src/lib.rs`: 두 플러그인 모두 `init()` 등록.
- `crates/oxiline-app/src-tauri/capabilities/default.json`: `"notification:default"`, `"opener:default"` permission 추가.
- `crates/oxiline-core/src/model.rs`: `SettingsSnapshot`에 `notifications_enabled: bool` 추가.
- `crates/oxiline-core/src/settings.rs`: `ensure_defaults()` / `snapshot()` 새 키 반영. `notifications_enabled` (default `false`), `notification_lead_minutes` (default `5`).
- `crates/oxiline-core/migrations/V2__phase2.sql` (신규): V1이 끝난 뒤 settings에 두 키 INSERT (ON CONFLICT DO NOTHING — idempotent).
- `crates/oxiline-core/src/db.rs`: `Migrations::new(vec![M::up(V1_INIT), M::up(V2_PHASE2)])`.
- `crates/oxiline-app/src-tauri/src/commands.rs`:
  - `request_notification_permission() -> Result<bool, String>` (specta)
  - `is_notification_permission_granted() -> Result<bool, String>` (specta)
  - `lib.rs`의 `collect_commands!`에 두 개 추가.
- `crates/oxiline-app/src/lib/api.ts`: `requestNotificationPermission` / `isNotificationPermissionGranted` 메서드 추가.
- `crates/oxiline-app/src/components/Preferences.tsx`: "알림" 섹션 추가. 토글 + 권한 상태 표시. 거부 시 `tauri-plugin-opener`로 `x-apple.systempreferences:com.apple.preference.notifications`를 열어 "macOS 시스템 설정 열기" 버튼 제공.

## 3. 백그라운드 알림 스케줄러

**목적**: 블록 시작 N분 전 macOS 알림 1회 발송.

**설계**:

- `crates/oxiline-app/src-tauri/src/notifier.rs` (신규):
  - `pub fn spawn_scheduler(app: AppHandle)` — 백그라운드 OS 스레드 시작.
  - 루프 (60초 주기):
    1. `state.conn()` 빌려 `timeline::get_now_context(&conn, now_minute_local())` 호출.
    2. `settings::get_bool(&conn, "notifications_enabled", false)`가 true면 진행, 아니면 다음 사이클.
    3. `next`가 있고 `next.starts_in_minute <= lead_minutes`이고 `last_notified.contains(next.id)`가 아니면 발송.
    4. `app_handle.notification().builder().title(next.title).body(body).show()?`.
    5. 발송한 id를 `last_notified` (in-memory `HashSet<String>`)에 push.
  - sleep/wake: 매 사이클 시작에서 `chrono::Local::now()`의 분 단위가 이전 사이클 대비 5분 이상 점프하면 `last_notified.clear()` (오래된 항목 재발송 가능).
  - 60초 sleep은 `std::thread::sleep(Duration::from_secs(60))` (Rust 2024, sync). 핸들 shutdown 신호는 단순 `Arc<AtomicBool>` 플래그.
- `crates/oxiline-app/src-tauri/src/lib.rs`: `setup`에서 `notifier::spawn_scheduler(app.handle().clone())`.
- `crates/oxiline-app/src/components/Preferences.tsx`: 토글 변경 시 `invoke('set_setting', {key, value})` — DB가 바뀌면 watcher가 `db-changed` emit, notifier는 다음 사이클에 새 값을 본다. 별도 reschedule 불요.
- 발송 body: "`{제목}`이(가) {N}분 후 시작돼요" (i18n 키 `notifier.body`).

## 4. 트레이 진행률 그리기

**목적**: §6.6의 3번째 재사용처 — 메뉴바 아이콘 자체에 하루 진행률.

**설계**:

- `crates/oxiline-app/src-tauri/Cargo.toml`: `image = { version = "0.25", default-features = false, features = ["png"] }` 추가 (Tauri 2의 tray icon은 PNG 바이트도 RGBA 바이트도 받음 — 안정성 위해 `image` 크레이트로 직접 RGBA 빌드 후 `tauri::image::Image::new_owned`에 전달).
- `crates/oxiline-app/src-tauri/src/tray.rs`:
  - `load_tray_icon()` → `render_progress_icon(progress: f32) -> tauri::image::Image<'static>`로 교체.
  - 22x22 RGBA 캔버스(`image::RgbaImage::new(22, 22)`), `--accent-oxide`(verdigris) 막대 배경 + 진행률 채움 + 끝에 도트 2px.
  - `progress = now_minute_local() / ((day_end - day_start) * 60)` 클램프 [0, 1].
- `crates/oxiline-app/src-tauri/src/lib.rs`:
  - `setup`에서 `app.listen("oxiline://db-changed", |_| tray::refresh(&handle))`은 이미 존재. 추가로 60초 주기 `std::thread::spawn`이 `tray::refresh(&handle)`을 호출.
  - 진행률 계산은 `tray::refresh` 내부에서.
- `tauri.conf.json` 변경 없음 (트레이 아이콘은 코드가 매번 새로 그림).
- **구현 노트**: Tauri 2의 `tauri::image::Image::new_owned(&rgba_bytes, w, h)`는 RGBA 4바이트 픽셀을 요구하므로 `image::RgbaImage::into_raw()`로 `Vec<u8>` 변환 후 전달. `Image`의 lifetime은 `'static` (소유).

## 5. 주간(Week) 뷰

**목적**: §7.3 — 7개 미니 세로 타임라인 가로 배치.

**설계**:

- `crates/oxiline-app/src/hooks.ts`: `useTimelineRange(from: string, to: string)` 추가.
  - `queryKey: qk.timelineRange(from, to)`.
  - `queryFn`: 7일을 `Promise.all`로 `api.getTimeline(date)` 호출 후 `{date, items}[]` 형태로 묶음.
- `crates/oxiline-app/src/lib/api.ts`: 이미 `getTimeline(date: string)`이 있으므로 그대로 재사용 (7번 호출).
- `crates/oxiline-app/src/components/WeekView.tsx` (신규):
  - 7개 컬럼 flex row. 각 컬럼은 `MiniDayTimeline` (압축 DayTimeline, `pxPerMin` 절반).
  - 각 컬럼 상단: 요일 + 일자 + 그 날 총 workload 분.
  - 클릭 시 그 날짜로 메인 뷰 전환 (`useUi.setDate(d)` + `useUi.setView("today")`).
  - 빈 컬럼은 "비어 있음" 안내.
- `crates/oxiline-app/src/App.tsx`: `WeekPlaceholder` 교체. `←`/`→` 핸들러에 `view === "today"` 가드.
- 백엔드 변경 없음. `timeline::get_timeline_for_date`는 단일 날짜만 받지만 7번 호출은 충분히 가벼움.

## 6. 드래그 앤 드롭 (블록 이동/리사이즈/백로그→타임라인)

**목적**: §7.1, §7.2 — Phase 2의 가장 큰 단일 작업.

**설계**:

- `crates/oxiline-app/package.json`: `@dnd-kit/core`, `@dnd-kit/utilities` 추가. `sortable`은 안 씀 (단순 reorder만 필요).
- `crates/oxiline-app/src/lib/dnd.ts` (신규): 공통 `DndProvider`와 세 가지 드래그 정의.
  - `DndContext`는 `DayTimeline`, `BacklogView`, `RoutineManager`의 공통 조상(`App.tsx`)에서 한 번 마운트.
  - **MoveBlock**:
    - `onDragEnd`에서 `over` 영역(다른 블록 또는 빈 슬롯 droppable)이 있으면:
      1. `id`가 `virtual:` 접두사면 Tauri 커맨드 `materialize_if_virtual`을 호출해 실 ID를 받는다.
      2. 새 `start_minute` 계산: drop Y 좌표 → 분 변환 → 5분 스냅.
      3. `updateTask({id: realId, start_minute, date?})`.
  - **ResizeBlock**:
    - 블록 하단 6px 영역에 `useDraggable` (axis: "y", axis lock). `data: {kind: "resize", item}`.
    - `onDragEnd`에서 새 `duration_minute = max(5, round((dropY - topY) / pxPerMin / 5) * 5)`.
  - **BacklogToTimeline**:
    - `BacklogView`의 각 행을 `useDraggable({id: task.id, data: {kind: "backlog", task}})`로.
    - `DayTimeline`의 빈 슬롯 영역을 `useDroppable({id: "slot-empty", data: {kind: "slot"}})`로.
    - `onDragEnd`에서 `updateTask({id: task.id, date, start_minute: 5분 스냅, duration_minute: 30 (기본)})`.
- `crates/oxiline-app/src-tauri/src/commands.rs`:
  - `materialize_if_virtual(id: String) -> Result<String, String>` (specta) — core의 이미 존재하는 `tasks::materialize_if_virtual`을 호출만 해주는 얇은 래퍼. 가상이면 실 ID 반환, 실 ID면 그대로.
  - `lib.rs`의 `collect_commands!`에 추가.
- `crates/oxiline-app/src/lib/api.ts`: `materializeIfVirtual` 메서드 추가.
- `crates/oxiline-app/src/components/BlockView.tsx`: dnd 통합, `useState`로 `transform` 추적, `style={{transform: CSS.Translate.toString(transform)}}`.
- `crates/oxiline-app/src/components/DayTimeline.tsx`: 빈 슬롯 droppable. 기존 클릭 → 인라인 추가는 `pointerdown` → `dblclick`으로 단순화(드래그와 충돌 방지). 또는: 드래그가 시작되면 클릭 핸들러는 무시.
- `crates/oxiline-app/src/components/BacklogView.tsx`: 각 행에 `useDraggable`.
- **키보드 대안** (드래그 없이 동등):
  - 포커스된 블록에서 `⌥↑/↓` (5분 단위 이동), `⌥⇧↑/↓` (15분), `⌘↑/↓` (리사이즈 5분), `⌘⇧↑/↓` (15분). 모두 Tauri `update_task` 호출.
  - `App.tsx`의 `useGlobalKeys`에 분기 추가 (focus가 `BlockView` 내부일 때만).
- `pnpm` / `bun` 둘 다 호환되도록 추가 — 현재 `bun` 사용 중이므로 `bun add @dnd-kit/core @dnd-kit/utilities`.

## 7. 루틴 그룹 CRUD + 인라인 편집 + 접근성 마무리

**목적**: §7.4 인라인 폼 + `routine_groups` UI 노출 + 키보드/ARIA/focus.

**설계**:

- `crates/oxiline-core/src/routines.rs`: `groups` 모듈 추가 (또는 별도 `routine_groups.rs`).
  ```rust
  pub fn list(conn: &Connection) -> Result<Vec<RoutineGroup>>;
  pub fn get(conn: &Connection, id: &str) -> Result<RoutineGroup>;
  pub fn create(conn: &Connection, name: String, icon: Option<String>) -> Result<RoutineGroup>;
  pub fn update(conn: &Connection, id: &str, name: Option<String>, icon: Option<Option<String>>, sort_order: Option<i32>) -> Result<RoutineGroup>;
  pub fn delete(conn: &Connection, id: &str) -> Result<()>;
  pub fn set_active(conn: &Connection, id: &str, active: bool) -> Result<RoutineGroup>;
  ```
- `crates/oxiline-app/src-tauri/src/commands.rs`:
  - `list_routine_groups()`, `create_routine_group(name, icon)`, `update_routine_group(id, patch)`, `delete_routine_group(id)`, `set_routine_group_active(id, active)`.
  - `set_routine_group_active` 내부에서 `UPDATE routine_blocks SET is_active=? WHERE group_id=?` (트랜잭션) 후 `Vec<RoutineBlock>` 반환 → 그룹 활성 토글이 자식 블록에 즉시 반영.
- `crates/oxiline-cli/src/{cli,main}.rs`: `routine group add/list/show/edit/rm/toggle` 서브커맨드 추가. `cli.rs`의 `RoutineAction`에 `Group` variant 추가, `main.rs`의 match에 핸들러 추가.
- `crates/oxiline-app/src/components/RoutineManager.tsx` (대폭 재작성):
  - 좌측: 그룹 사이드바 (스크롤 가능). 그룹 추가 버튼(`+`), 각 그룹: 이름, 활성 토글, 클릭 시 선택. 선택된 그룹 우측에 표시.
  - 우측: 선택 그룹의 블록 리스트. 각 행:
    - 1줄: 색 도트 · 제목 · 시각 · 소요시간.
    - 2줄: 요일 배지 + ⋯ 메뉴.
    - **행 클릭 → 인라인 폼 펼쳐짐**: 제목, 시작(`<input type="time">`), 종료 또는 duration(`<input type="number">`), 7요일 토글 + 3 프리셋, 카테고리 select, 메모 textarea, 저장/취소/삭제 버튼.
    - `Esc`로 폼 닫기, `⌘S`로 저장.
  - 인라인 폼은 `useState`로 expanded id 1개만 추적. 키보드 흐름이 끊기지 않음.
- `crates/oxiline-app/src/lib/api.ts`: 그룹 5개 메서드 추가.
- `crates/oxiline-app/src/hooks.ts`: `useRoutineGroups`, `useCreateRoutineGroup`, `useUpdateRoutineGroup`, `useDeleteRoutineGroup`, `useSetRoutineGroupActive` 추가.

**접근성 마무리** (전 컴포넌트):

- 모든 `<button>`에 `aria-label` (i18n 키, ko/en 동시 적용). 아이콘 전용 버튼(⋯, ✕)은 필수.
- 인터랙티브 블록은 `role="button"`, 토글은 `role="checkbox"` + `aria-checked`.
- 포커스 표시: `styles.css`의 기존 `::focus-visible` 규칙 활용. dnd-kit의 tabIndex 0 유지.
- 모달 없음 (전부 인라인/슬라이드 오버) → 포커스 트랩 불요.
- 색만으로 정보 전달 금지(이미 §6.9에서 이중 인코딩). 변경 없음.

## 변경 파일 목록

### 신규
- `crates/oxiline-app/src/components/WeekView.tsx`
- `crates/oxiline-app/src-tauri/src/notifier.rs`
- `crates/oxiline-app/src/lib/dnd.ts`
- `crates/oxiline-core/src/routine_groups.rs` (또는 routines.rs에 모듈화)
- `crates/oxiline-core/migrations/V2__phase2.sql`
- `docs/superpowers/specs/2026-07-30-phase2-polish-design.md` (이 문서)

### 수정
- `crates/oxiline-app/src-tauri/Cargo.toml` (의존성 4: `tauri-nspanel`, `tauri-plugin-notification`, `tauri-plugin-opener`, `image`)
- `crates/oxiline-app/src-tauri/capabilities/default.json` (notification + opener permission)
- `crates/oxiline-app/package.json` (의존성 2: `@dnd-kit/core`, `@dnd-kit/utilities`)
- `crates/oxiline-app/src/lib/{api,store}.ts`
- `crates/oxiline-app/src/hooks.ts`
- `crates/oxiline-app/src/components/{DayTimeline,BacklogView,BlockView,RoutineManager,Preferences,Header}.tsx`
- `crates/oxiline-app/src/locales/{ko,en}.json` (신규 키)
- `crates/oxiline-core/src/{db,model,settings}.rs`
- `crates/oxiline-core/src/routines.rs` (group 모듈 추가)
- `crates/oxiline-cli/src/{cli,main}.rs` (group 서브커맨드)

### 변경 없음 (의도적)
- `crates/oxiline-core/src/timeline.rs`, `tasks.rs` (드래그 후 update_task가 idempotent하므로 그대로)
- `crates/oxiline-core/src/{error,paths,util,db,categories}.rs`
- `crates/oxiline-core/migrations/V1__init.sql`
- `crates/oxiline-core/tests/timeline.rs` (테스트는 별도 라운드)
- 문서 `01-08` (스펙이 변경된 것은 아니므로)

## 비목표 (이번 라운드에서 의도적으로 안 함)

- `tauri-nspanel` 비-macOS 빌드 (CI 매트릭스 추가 없음, 비-macOS는 기존 오버레이 폴백)
- dnd-kit의 키보드 센서리 (마우스 + 위 키보드 단축키로 동등 기능 확보)
- 습관 스트릭/점수 (Non-goal)
- MCP serve (Phase 3)
- EventKit / Shortcuts 연동 (Phase 3)
- 코드 서명/공증 자동화 (Phase 3)
- **테스트 작성** (사용자 지시: 모든 구현 후 별도 라운드)

## 완료 검증 (사용자 수동)

- macOS GUI 부팅 → 다른 앱 위에서 `⌘⇧O` → HUD 표시 + 커서 유지 (1번)
- 환경설정 → 알림 토글 ON → 권한 요청 → 다음 블록 5분 전 macOS 알림 1회 (2, 3번)
- 메뉴바 아이콘에 진행률 바가 시간 흐름에 따라 채워짐 (4번)
- `2` 키로 주간 뷰 → 7개 미니 타임라인 표시 → 컬럼 클릭 시 오늘 뷰로 점프 (5번)
- 타임라인 블록 드래그 → 5분 스냅으로 이동 / 하단 핸들 드래그 → 리사이즈 / 백로그 행을 타임라인으로 드래그 → 그 시각 30분 항목으로 스케줄 (6번)
- 루틴 관리 → 그룹 추가/이름변경/삭제/토글, 블록 인라인 편집, 키보드만으로 모든 조작 가능 (7번)
