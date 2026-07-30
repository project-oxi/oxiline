# Phase 2 다듬기 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Phase 1(MVP)에 이어 §7 §8의 7개 Phase 2 항목(tauri-nspanel HUD, 알림, 트레이 진행률, 주간 뷰, 드래그 앤 드롭, 루틴 그룹+인라인 편집+접근성)을 구현한다.

**Architecture:** Tauri 2 + React 19 + Vite + oxiline-core SQLite. 의존 그래프대로 1→2→3 (네이티브), 4 (트레이), 5 (주간), 6 (드래그), 7 (그룹+접근성) 순서로 진행. 각 작업은 빌드/린트/스모크 검증 + 커밋으로 끝난다.

**Tech Stack:**
- Rust 2024, Tauri 2.11, rusqlite 0.32, rusqlite_migration 1.3
- React 19, TypeScript 5.7, Vite 6, Tailwind v4
- 신규 Rust 의존성: `tauri-nspanel`(git, branch=v1), `tauri-plugin-notification 2`, `tauri-plugin-opener 2`, `image 0.25`
- 신규 JS 의존성: `@dnd-kit/core`, `@dnd-kit/utilities`
- 패키지 매니저: `bun`(확정, pnpm 사용 안 함)

**참조 문서:**
- `docs/superpowers/specs/2026-07-30-phase2-polish-design.md` — 설계의 단일 진실. 모든 분기는 거기서.
- `doc/01-08*.md` — 제품/UX/데이터/아키텍처/CLI/디자인/UI 스펙
- `crates/oxiline-core/migrations/V1__init.sql` — 기존 스키마

## Global Constraints

- **테스트는 이번 라운드에서 작성하지 않는다** (사용자 지시). 통합 테스트 12개는 그대로 유지하고 회귀가 없는지만 확인. 새 테스트는 별도 라운드.
- **커밋 컨벤션**: `feat(...)`, `fix(...)`, `chore(...)`, `docs(...)` — 기존 커밋 로그와 동일.
- **i18n**: 모든 사용자 가시 문자열은 `ko.json`/`en.json` 키로 추가. 하드코드 한국어/영어 금지.
- **하드코딩 금지**: 모든 시간은 `oxiline_core::util::now_minute_local()` / `today_local()`을 통한다. 모든 DB 경로는 `oxiline_core::paths::db_path()`.
- **i18n 키 네이밍**: 점-구분 네임스페이스(`notifier.body`, `week.workload`, `routineGroup.sidebar`).
- **빌드 검증**: 작업 종료 시 `cargo build --workspace` 통과 필수. 프론트 변경 시 `bun run build`도 통과.
- **non-async 코어**: `oxiline-core`는 sync 그대로. Tauri 명령 안에서만 `spawn_blocking` 또는 직접 호출.
- **한국 시간 표시**: 모든 사용자 표시 시간은 24시간제 (12시간제 AM/PM 배지 금지 — §6.3).
- **UI 카피 언어**: 메시지 박스, 토스트, 빈 상태, 에러는 i18n. 코드 주석은 한국어 가능.
- **이름 충돌 회피**: 시맨틱 색 hue(35, 189)는 카테고리 팔레트에서 제외(§6.2).
- **한 명령 = 한 커밋**: 각 작업의 마지막 step은 단일 `git commit`.

---

## Task 1: V2 마이그레이션 (Phase 2 settings 키 시드)

**Files:**
- Create: `crates/oxiline-core/migrations/V2__phase2.sql`
- Modify: `crates/oxiline-core/src/db.rs`
- Modify: `crates/oxiline-core/src/model.rs`
- Modify: `crates/oxiline-core/src/settings.rs`

**Interfaces:**
- Consumes: 기존 `SettingsSnapshot` 구조
- Produces: `SettingsSnapshot`에 `notifications_enabled: bool`, `notification_lead_minutes: u32` 두 필드 추가

- [ ] **Step 1: `V2__phase2.sql` 작성**

```sql
-- OxiLine Phase 2 settings additions (2026-07-30 spec §2).
-- All time columns remain LOCAL wall-clock minute-of-day integers.

INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES
    ('notifications_enabled',     'false', '2026-07-30T00:00:00Z'),
    ('notification_lead_minutes', '5',     '2026-07-30T00:00:00Z');
```

`crates/oxiline-core/migrations/V2__phase2.sql`로 저장.

- [ ] **Step 2: `db.rs`에 V2 등록**

`crates/oxiline-core/src/db.rs` 수정:

```rust
const V1_INIT: &str = include_str!("../migrations/V1__init.sql");
const V2_PHASE2: &str = include_str!("../migrations/V2__phase2.sql");

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(V1_INIT), M::up(V2_PHASE2)])
}
```

- [ ] **Step 3: `model.rs`의 `SettingsSnapshot`에 두 필드 추가**

`crates/oxiline-core/src/model.rs`의 `SettingsSnapshot` 구조에 다음 두 필드 추가 (specta::Type 파생 이미 있음):

```rust
pub notifications_enabled: bool,
pub notification_lead_minutes: u32,
```

- [ ] **Step 4: `settings.rs`의 `snapshot()` 함수 갱신**

`crates/oxiline-core/src/settings.rs`의 `snapshot()`에 두 키 추가:

```rust
notifications_enabled: get_bool(conn, "notifications_enabled", false),
notification_lead_minutes: get_i64(conn, "notification_lead_minutes", 5) as u32,
```

- [ ] **Step 5: 빌드 확인 + doctor로 마이그레이션 검증**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline
cargo build --workspace
OXILINE_DB_PATH=/tmp/oxiline-v2-smoke.db /Volumes/MERCURY/PROJECTS/oxiline/target/debug/oxiline doctor
```

기대 출력: `Schema version: 2 (latest)`. `settings get` (CLI)에 `notifications_enabled`와 `notification_lead_minutes` 두 키가 보여야 함.

```bash
OXILINE_DB_PATH=/tmp/oxiline-v2-smoke.db /Volumes/MERCURY/PROJECTS/oxiline/target/debug/oxiline settings get --json
```

기대: `{"...": "...", "notifications_enabled": false, "notification_lead_minutes": 5, ...}`. 임시 DB는 `rm -f /tmp/oxiline-v2-smoke.db /tmp/oxiline-v2-smoke.db-*`로 정리.

- [ ] **Step 6: 기존 통합 테스트 회귀 확인**

```bash
cargo test -p oxiline-core --tests
```

기대: 12/12 통과 (변경이 기존 동작에 영향 없음).

- [ ] **Step 7: 커밋**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline
git add crates/oxiline-core/migrations/V2__phase2.sql \
        crates/oxiline-core/src/db.rs \
        crates/oxiline-core/src/model.rs \
        crates/oxiline-core/src/settings.rs
git -c user.email=oxi@local -c user.name=Oxi commit -m "feat(core): V2 migration + notifications settings"
```

---

## Task 2: tauri-nspanel HUD 마이그레이션

**Files:**
- Modify: `crates/oxiline-app/src-tauri/Cargo.toml`
- Modify: `crates/oxiline-app/src-tauri/Cargo.toml` (build dependency는 변경 없음)
- Modify: `crates/oxiline-app/src-tauri/src/lib.rs`
- Modify: `crates/oxiline-app/src-tauri/src/hud.rs`

**Interfaces:**
- Consumes: 기존 `hud::show(&AppHandle)` (lib.rs의 `shortcuts::register_default`에서 호출)
- Produces: `hud::init_panel(&AppHandle) -> tauri::Result<()>` 신규. macOS면 `to_panel()`로 NSPanel 변환, 비-macOS면 no-op.

- [ ] **Step 1: Cargo.toml에 tauri-nspanel 추가**

`crates/oxiline-app/src-tauri/Cargo.toml`의 `[dependencies]` 섹션에 추가:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
tauri-nspanel = { git = "https://github.com/ahkohd/tauri-nspanel", branch = "v1" }
```

(전체 파일이 아닌 `[target...]` 블록만 추가. 기존 `[dependencies]`는 그대로.)

- [ ] **Step 2: `hud.rs`에 `init_panel` 함수 추가**

`crates/oxiline-app/src-tauri/src/hud.rs` 끝에 추가:

```rust
/// macOS: convert the `hud` window to a non-activating NSPanel so it
/// never steals focus from the foreground app (§4.4, Phase 2 spec §1).
/// Non-macOS: no-op (the existing transparent overlay already works).
pub fn init_panel(app: &AppHandle) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    {
        use tauri_nspanel::{WindowExt, raw_nspanel::RawNSPanel, raw_nspanel::StyleMask, raw_nspanel::PanelLevel, raw_nspanel::CollectionBehavior};
        let Some(win) = app.get_webview_window("hud") else {
            return Ok(());
        };
        let panel = win.to_panel()?;
        // NSWindowStyleMask::HUDWindow = 0x1 | NSWindowStyleMask::Borderless = 0
        // Use bitflags: 0x1 = NSWindowStyleMaskHUDWindow
        panel.set_style_mask(1);
        // Floating level (above normal windows, below modal)
        // NSStatusWindowLevel = 25 in cocoa, but use NSPanelLevel::Floating for Tauri-friendly access.
        // Raw value: 3 = NSFloatingWindowLevel
        panel.set_level(3);
        // Don't hide when the app deactivates (so HUD stays when user types in another app)
        panel.set_hides_on_deactivate(false);
        // Show on every Space + above fullscreen apps
        // NSWindowCollectionBehaviorFullScreenAuxiliary = 1 << 5 = 32
        // NSWindowCollectionBehaviorCanJoinAllSpaces = 1 << 0 = 1
        // NSWindowCollectionBehaviorTransient = 1 << 2 = 4
        panel.set_collection_behavior(1 | 4 | 32);
        // Initially hidden
        let _ = win.hide();
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}
```

참고: 만약 `raw_nspanel` 모듈이 export되지 않으면 `tauri_nspanel`의 docs에서 정확한 import 경로 확인. `panel.set_*` 메서드는 `tauri_nspanel::Panel` 트레이트.

- [ ] **Step 3: `lib.rs`에 플러그인 + init 호출**

`crates/oxiline-app/src-tauri/src/lib.rs`의 `tauri::Builder::default()`에 `.plugin(tauri_nspanel::init())` 추가 (macOS만). `setup` 클로저 시작 부분에 `hud::init_panel(app.handle())?;` 추가. macOS 게이팅:

```rust
#[cfg(target_os = "macos")]
.plugin(tauri_nspanel::init())
```

setup 내부:
```rust
#[cfg(target_os = "macos")]
{
    if let Err(e) = hud::init_panel(app.handle()) {
        eprintln!("oxiline: hud init_panel failed: {e}");
    }
}
```

- [ ] **Step 4: 빌드 확인**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline
cargo build -p oxiline-app 2>&1 | tail -20
```

기대: `Finished` 라인. 에러가 `raw_nspanel` 모듈 import 관련이면, `crates/oxiline-app/src-tauri/Cargo.toml`은 그대로 두고 `hud.rs`의 import만 다음으로 교체:

```rust
use tauri_nspanel::{Panel, WindowExt};
```

(타입은 `tauri_nspanel`이 재노출.)

- [ ] **Step 5: 커밋**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline
git add crates/oxiline-app/src-tauri/Cargo.toml \
        crates/oxiline-app/src-tauri/src/hud.rs \
        crates/oxiline-app/src-tauri/src/lib.rs
git -c user.email=oxi@local -c user.name=Oxi commit -m "feat(app): tauri-nspanel 기반 non-activating HUD"
```

---

## Task 3: tauri-plugin-notification + tauri-plugin-opener

**Files:**
- Modify: `crates/oxiline-app/src-tauri/Cargo.toml`
- Modify: `crates/oxiline-app/src-tauri/src/lib.rs`
- Modify: `crates/oxiline-app/src-tauri/capabilities/default.json`
- Modify: `crates/oxiline-app/src-tauri/src/commands.rs`
- Modify: `crates/oxiline-app/src/lib/api.ts`
- Modify: `crates/oxiline-app/src/components/Preferences.tsx`
- Modify: `crates/oxiline-app/src/locales/ko.json`
- Modify: `crates/oxiline-app/src/locales/en.json`

**Interfaces:**
- Consumes: 기존 `settings::snapshot()` (notifications_enabled, notification_lead_minutes)
- Produces: Tauri commands `request_notification_permission`, `is_notification_permission_granted`. JS methods `api.requestNotificationPermission()`, `api.isNotificationPermissionGranted()`. Preferences에 "알림" 섹션.

- [ ] **Step 1: Cargo.toml에 두 플러그인 추가**

`crates/oxiline-app/src-tauri/Cargo.toml`의 `[dependencies]`에:

```toml
tauri-plugin-notification = "2"
tauri-plugin-opener = "2"
```

- [ ] **Step 2: `lib.rs`에 플러그인 등록**

`crates/oxiline-app/src-tauri/src/lib.rs`의 `tauri::Builder::default()` 체인에 추가 (다른 `.plugin(...)` 호출과 같은 위치):

```rust
.plugin(tauri_plugin_notification::init())
.plugin(tauri_plugin_opener::init())
```

- [ ] **Step 3: `capabilities/default.json`에 permission 추가**

`crates/oxiline-app/src-tauri/capabilities/default.json`의 `permissions` 배열에 추가:

```json
"notification:default",
"opener:default"
```

- [ ] **Step 4: `commands.rs`에 두 커맨드 추가**

`crates/oxiline-app/src-tauri/src/commands.rs` 끝에:

```rust
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
#[specta::specta]
pub async fn request_notification_permission(
    app: tauri::AppHandle,
) -> Result<bool, String> {
    use tauri_plugin_notification::PermissionState;
    match app.notification().request_permission() {
        Ok(PermissionState::Granted) => Ok(true),
        Ok(PermissionState::Denied) => Ok(false),
        Ok(_) => Ok(false),
        Err(e) => Err(format!("notification:request_permission: {e}")),
    }
}

#[tauri::command]
#[specta::specta]
pub fn is_notification_permission_granted(
    app: tauri::AppHandle,
) -> Result<bool, String> {
    use tauri_plugin_notification::PermissionState;
    match app.notification().permission_state() {
        Ok(PermissionState::Granted) => Ok(true),
        _ => Ok(false),
    }
}

#[tauri::command]
#[specta::specta]
pub fn open_notification_settings(app: tauri::AppHandle) -> Result<(), String> {
    app.opener()
        .open_path("x-apple.systempreferences:com.apple.preference.notifications", None::<&str>)
        .map_err(|e| format!("opener: {e}"))
}
```

- [ ] **Step 5: `lib.rs`의 `collect_commands!` 매크로에 추가**

`crates/oxiline-app/src-tauri/src/lib.rs`의 `collect_commands![...]` 배열에 세 커맨드 이름 추가:

```rust
commands::request_notification_permission,
commands::is_notification_permission_granted,
commands::open_notification_settings,
```

- [ ] **Step 6: `api.ts`에 메서드 추가**

`crates/oxiline-app/src/lib/api.ts`의 `api` 객체에 추가:

```ts
requestNotificationPermission: () => invoke<boolean>("request_notification_permission"),
isNotificationPermissionGranted: () => invoke<boolean>("is_notification_permission_granted"),
openNotificationSettings: () => invoke<void>("open_notification_settings"),
```

- [ ] **Step 7: i18n 키 추가**

`crates/oxiline-app/src/locales/ko.json`에 추가:

```json
"notifications": {
  "section": "알림",
  "enable": "블록 시작 전 알림 보내기",
  "enableHelp": "다음 항목이 곧 시작될 때 macOS 알림을 보냅니다",
  "leadMinutes": "몇 분 전",
  "requestPermission": "권한 요청",
  "openSystemSettings": "macOS 시스템 설정 열기",
  "granted": "권한 허용됨",
  "denied": "권한 거부됨 — 시스템 설정에서 켜주세요"
}
```

`en.json`에 동등 항목:

```json
"notifications": {
  "section": "Notifications",
  "enable": "Notify me before a block starts",
  "enableHelp": "Show a macOS notification when the next block is about to start",
  "leadMinutes": "Minutes before",
  "requestPermission": "Request permission",
  "openSystemSettings": "Open macOS System Settings",
  "granted": "Permission granted",
  "denied": "Permission denied — enable in System Settings"
}
```

(기존 JSON 구조가 `{ "common": {...}, "palette": {...} }` 형태이므로 같은 레벨에 `notifications` 키 추가.)

- [ ] **Step 8: `Preferences.tsx`에 알림 섹션 추가**

`crates/oxiline-app/src/components/Preferences.tsx`의 기존 섹션 리스트(타이포그래피/타임라인/단축키/카테고리/데이터/정보)에 "알림" 섹션을 추가. 기존 토글 UI 패턴을 그대로 따른다:

```tsx
import { useTranslation } from "react-i18next";
import { api } from "../lib/api";
import { useQuery, useMutation } from "@tanstack/react-query";
import { useSetSetting } from "../hooks";

// Preferences 컴포넌트 안:
const { t } = useTranslation();
const setSetting = useSetSetting();
const permQ = useQuery({
  queryKey: ["notification-permission"],
  queryFn: () => api.isNotificationPermissionGranted(),
  staleTime: 5000,
});
const requestPerm = useMutation({
  mutationFn: () => api.requestNotificationPermission(),
  onSuccess: () => permQ.refetch(),
});

// ... 기존 섹션 다음에 추가:
<section>
  <h3>{t("notifications.section")}</h3>
  <label>
    <input
      type="checkbox"
      checked={settings.notifications_enabled}
      onChange={(e) =>
        setSetting.mutate({ key: "notifications_enabled", value: e.target.checked ? "true" : "false" })
      }
    />
    {t("notifications.enable")}
  </label>
  <p>{t("notifications.enableHelp")}</p>
  <label>
    {t("notifications.leadMinutes")}: {settings.notification_lead_minutes}
    <input
      type="range"
      min={1}
      max={30}
      value={settings.notification_lead_minutes}
      onChange={(e) =>
        setSetting.mutate({ key: "notification_lead_minutes", value: e.target.value })
      }
    />
  </label>
  {permQ.data ? (
    <p>{t("notifications.granted")}</p>
  ) : (
    <>
      <p>{t("notifications.denied")}</p>
      <button onClick={() => requestPerm.mutate()}>{t("notifications.requestPermission")}</button>
      <button onClick={() => api.openNotificationSettings()}>
        {t("notifications.openSystemSettings")}
      </button>
    </>
  )}
</section>
```

(`settings`는 기존 useSettings() 훅 결과, `useSetSetting`은 hooks.ts에서 이미 export됨.)

- [ ] **Step 9: 프론트엔드 빌드 검증**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline/crates/oxiline-app
bun run build
```

기대: `dist/` 갱신, 에러 없음. 만약 specta 타입이 `bindings.ts`에 반영 안 됐으면 `cargo build -p oxiline-app`로 bindings 재生成.

- [ ] **Step 10: 커밋**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline
git add crates/oxiline-app/src-tauri/Cargo.toml \
        crates/oxiline-app/src-tauri/src/lib.rs \
        crates/oxiline-app/src-tauri/capabilities/default.json \
        crates/oxiline-app/src-tauri/src/commands.rs \
        crates/oxiline-app/src/lib/api.ts \
        crates/oxiline-app/src/components/Preferences.tsx \
        crates/oxiline-app/src/locales/ko.json \
        crates/oxiline-app/src/locales/en.json
git -c user.email=oxi@local -c user.name=Oxi commit -m "feat(app): notification permission + preferences UI"
```

---

## Task 4: 백그라운드 알림 스케줄러

**Files:**
- Create: `crates/oxiline-app/src-tauri/src/notifier.rs`
- Modify: `crates/oxiline-app/src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `AppState::conn()` (DB 풀에서 빌림), `timeline::get_now_context()`, `settings::get_bool`, `settings::get_i64`, `tauri_plugin_notification::NotificationExt`
- Produces: `pub fn spawn_scheduler(app: AppHandle)` — 한 번 호출하면 프로세스 수명 동안 동작하는 OS 스레드 시작

- [ ] **Step 1: `notifier.rs` 생성**

`crates/oxiline-app/src-tauri/src/notifier.rs`:

```rust
//! Background notification scheduler (Phase 2 spec §3).
//!
//! Polls every 60s. When a block is about to start, posts a one-shot
//! macOS notification. Keeps a memory-only `last_notified` set so the
//! same item is only notified once per scheduler lifetime.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::state::AppState;

const POLL_INTERVAL: Duration = Duration::from_secs(60);
const SLEEP_GAP_MINUTES: i64 = 5;

pub fn spawn_scheduler(app: AppHandle) {
    std::thread::Builder::new()
        .name("oxiline-notifier".into())
        .spawn(move || {
            let last_notified: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
            let mut last_now_minute: i64 = -1;

            loop {
                std::thread::sleep(POLL_INTERVAL);

                let state = app.state::<AppState>();
                let conn = state.conn();
                let enabled = oxiline_core::settings::get_bool(&conn, "notifications_enabled", false);
                if !enabled {
                    continue;
                }

                let now_minute = oxiline_core::util::now_minute_local() as i64;
                // Sleep/wake: if wall time jumped significantly, drop the dedup set.
                if last_now_minute >= 0 && (now_minute - last_now_minute).abs() > SLEEP_GAP_MINUTES * 60 {
                    last_notified.lock().clear();
                }
                last_now_minute = now_minute;

                let lead = oxiline_core::settings::get_i64(&conn, "notification_lead_minutes", 5);
                let Ok(ctx) = oxiline_core::timeline::get_now_context(&conn, now_minute as u16) else {
                    continue;
                };
                let Some(next) = ctx.next else { continue };
                let Some(starts_in) = next.starts_in_minute else { continue };
                if starts_in > lead as u32 {
                    continue;
                }
                if last_notified.lock().contains(&next.id) {
                    continue;
                }

                let body = format!("{}분 후 시작돼요", starts_in);
                let _ = app
                    .notification()
                    .builder()
                    .title(&next.title)
                    .body(&body)
                    .show();
                last_notified.lock().insert(next.id);
            }
        })
        .expect("failed to spawn notifier thread");
}
```

- [ ] **Step 2: `lib.rs`에 모듈 등록 + setup 호출**

`crates/oxiline-app/src-tauri/src/lib.rs`의 `mod` 블록에 `mod notifier;` 추가. `setup` 클로저 안, `watcher::spawn` 다음 줄에:

```rust
notifier::spawn_scheduler(app.handle().clone());
```

- [ ] **Step 3: 빌드 확인**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline
cargo build -p oxiline-app 2>&1 | tail -20
```

기대: `Finished`. 만약 `tauri_plugin_notification::NotificationExt`가 import 안 되면 `use tauri_plugin_notification::NotificationExt as _;` 형태로 사용.

- [ ] **Step 4: 커밋**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline
git add crates/oxiline-app/src-tauri/src/notifier.rs \
        crates/oxiline-app/src-tauri/src/lib.rs
git -c user.email=oxi@local -c user.name=Oxi commit -m "feat(app): background notification scheduler"
```

---

## Task 5: 트레이 진행률 그리기

**Files:**
- Modify: `crates/oxiline-app/src-tauri/Cargo.toml`
- Modify: `crates/oxiline-app/src-tauri/src/tray.rs`
- Modify: `crates/oxiline-app/src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `tray::build(&AppHandle)`, `tray::refresh(&AppHandle)` (이미 존재)
- Produces: `tray::render_progress_icon(progress: f32) -> tauri::image::Image<'static>` — 22×22 RGBA, `--accent-oxide` 색 진행률 막대

- [ ] **Step 1: Cargo.toml에 `image` 크레이트 추가**

```toml
image = { version = "0.25", default-features = false, features = ["png"] }
```

- [ ] **Step 2: `tray.rs`에 진행률 아이콘 렌더링 추가**

기존 `load_tray_icon` 함수를 다음으로 교체. 파일 상단에 import 추가:

```rust
use image::{ImageBuffer, Rgba};
```

`load_tray_icon`을 호출하는 곳을 변경:

```rust
// 22x22 progress icon. 0 = no progress (small dot), 1 = full bar.
const SIZE: u32 = 22;
const BAR_PAD: u32 = 4; // top/bottom margin for the bar
const BAR_HEIGHT: u32 = SIZE - BAR_PAD * 2;
const DOT_RADIUS: u32 = 2;

// Verdigris (oxide) — pulled from styles.css values.
const OXIDE_R: u8 = 88;
const OXIDE_G: u8 = 167;
const OXIDE_B: u8 = 161;
const OXIDE_A: u8 = 255;

pub fn render_progress_icon(progress: f32) -> tauri::image::Image<'static> {
    let p = progress.clamp(0.0, 1.0);
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(SIZE, SIZE, Rgba([0, 0, 0, 0]));

    // Bar background: dim outline
    for y in BAR_PAD..(SIZE - BAR_PAD) {
        for x in 2..(SIZE - 2) {
            img.put_pixel(x, y, Rgba([128, 128, 128, 80]));
        }
    }
    // Bar fill
    let fill_w = ((SIZE - 4) as f32 * p) as u32;
    for y in BAR_PAD..(SIZE - BAR_PAD) {
        for x in 2..(2 + fill_w) {
            img.put_pixel(x, y, Rgba([OXIDE_R, OXIDE_G, OXIDE_B, OXIDE_A]));
        }
    }
    // Leading dot (only if progress > 0)
    if p > 0.0 && fill_w > 0 {
        let cx = (2 + fill_w - 1) as i32;
        let cy = (SIZE / 2) as i32;
        for dy in -(DOT_RADIUS as i32)..=DOT_RADIUS as i32 {
            for dx in -(DOT_RADIUS as i32)..=DOT_RADIUS as i32 {
                if dx * dx + dy * dy <= (DOT_RADIUS as i32).pow(2) {
                    let x = (cx + dx) as u32;
                    let y = (cy + dy) as u32;
                    if x < SIZE && y < SIZE {
                        img.put_pixel(x, y, Rgba([255, 255, 255, 255]));
                    }
                }
            }
        }
    }

    let rgba = img.into_raw();
    tauri::image::Image::new_owned(rgba, SIZE, SIZE)
}
```

`build()` 함수 안에서 `load_tray_icon()` 호출을 `render_progress_icon(0.0)`으로 교체.

- [ ] **Step 3: `tray::refresh`에서 진행률 계산 + 갱신**

`crates/oxiline-app/src-tauri/src/tray.rs`의 `refresh` 함수를 갱신:

```rust
pub fn refresh(app: &AppHandle) {
    let Some(tray) = app.tray_by_id("main") else { return };
    let state = app.state::<AppState>();
    let conn = state.conn();
    let now_min = oxiline_core::util::now_minute_local() as f32;
    let day_start = oxiline_core::settings::get_i64(&conn, "day_start_hour", 5) as f32 * 60.0;
    let day_end = oxiline_core::settings::get_i64(&conn, "day_end_hour", 26) as f32 * 60.0;
    let progress = ((now_min - day_start) / (day_end - day_start)).clamp(0.0, 1.0);
    let _ = tray.set_icon(Some(render_progress_icon(progress)));
    // (메뉴의 "지금" 텍스트도 갱신하려면 기존 build_menu 로직 재호출)
    if let Ok(menu) = build_menu(app) {
        let _ = tray.set_menu(Some(menu));
    }
}
```

기존 `refresh`의 본문이 `build_menu`만 재호출이라면 위 형태로 교체. `tray_by_id`의 ID는 `build()`에서 `TrayIconBuilder::with_id("main")`이 사용됐는지 확인 후 그 ID로 교체.

- [ ] **Step 4: `lib.rs`의 60초 트레이 갱신 스레드 추가**

`crates/oxiline-app/src-tauri/src/lib.rs`의 `setup` 클로저 끝, `Ok(())` 직전에:

```rust
let h = app.handle().clone();
std::thread::spawn(move || loop {
    std::thread::sleep(std::time::Duration::from_secs(60));
    crate::tray::refresh(&h);
});
```

- [ ] **Step 5: 빌드 확인**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline
cargo build -p oxiline-app 2>&1 | tail -20
```

기대: `Finished`. `tray.set_icon`이 `Option<Image>`를 받는 시그니처는 Tauri 2.11 기준. 컴파일 에러가 `&Image` vs `Image` 차이면 `Some(&icon)` 또는 `Some(icon)` 둘 중 컴파일러가 요구하는 형태로 조정.

- [ ] **Step 6: 커밋**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline
git add crates/oxiline-app/src-tauri/Cargo.toml \
        crates/oxiline-app/src-tauri/src/tray.rs \
        crates/oxiline-app/src-tauri/src/lib.rs
git -c user.email=oxi@local -c user.name=Oxi commit -m "feat(app): tray icon progress bar"
```

---

## Task 6: 주간(Week) 뷰

**Files:**
- Modify: `crates/oxiline-app/src/hooks.ts`
- Modify: `crates/oxiline-app/src/components/WeekView.tsx` (create)
- Modify: `crates/oxiline-app/src/App.tsx`
- Modify: `crates/oxiline-app/src/locales/ko.json`
- Modify: `crates/oxiline-app/src/locales/en.json`

**Interfaces:**
- Consumes: `api.getTimeline(date: string)`, `useUi` (Zustand store)
- Produces: `useTimelineRange(from: string, to: string)` 훅, `WeekView` 컴포넌트

- [ ] **Step 1: i18n 키 추가**

`ko.json`에:

```json
"week": {
  "title": "주간",
  "today": "오늘",
  "workload": "{{n}}분",
  "empty": "비어 있음",
  "dayHeader": "{{weekday}} {{date}}"
}
```

`en.json`에 동등:

```json
"week": {
  "title": "Week",
  "today": "Today",
  "workload": "{{n}} min",
  "empty": "Empty",
  "dayHeader": "{{weekday}} {{date}}"
}
```

- [ ] **Step 2: `useTimelineRange` 훅 추가**

`crates/oxiline-app/src/hooks.ts` 끝에:

```ts
export function useTimelineRange(from: string, to: string) {
  const dates = useMemo(() => {
    const out: string[] = [];
    const [y, m, d] = from.split("-").map(Number);
    let dt = new Date(y, m - 1, d);
    const [y2, m2, d2] = to.split("-").map(Number);
    const end = new Date(y2, m2 - 1, d2);
    while (dt <= end) {
      out.push(
        `${dt.getFullYear()}-${String(dt.getMonth() + 1).padStart(2, "0")}-${String(
          dt.getDate(),
        ).padStart(2, "0")}`,
      );
      dt = new Date(dt.getFullYear(), dt.getMonth(), dt.getDate() + 1);
    }
    return out;
  }, [from, to]);

  return useQuery({
    queryKey: qk.timelineRange(from, to),
    queryFn: async () => {
      const results = await Promise.all(
        dates.map(async (date) => ({ date, items: await api.getTimeline(date) })),
      );
      return results;
    },
  });
}
```

`qk` 객체에 `timelineRange: (from: string, to: string) => ["timeline-range", from, to] as const` 추가.

- [ ] **Step 3: `WeekView.tsx` 생성**

`crates/oxiline-app/src/components/WeekView.tsx`:

```tsx
import { useTranslation } from "react-i18next";
import { useUi } from "../lib/store";
import { useTimelineRange } from "../hooks";
import { addDays, todayStr } from "../lib/store";
import type { TimelineItem } from "../types";

function weekdayLabel(date: string, locale: string): string {
  const d = new Date(date);
  return d.toLocaleDateString(locale, { weekday: "short" });
}

function monthDay(date: string, locale: string): string {
  const d = new Date(date);
  return d.toLocaleDateString(locale, { month: "short", day: "numeric" });
}

function MiniDay({
  date,
  items,
  onJump,
}: {
  date: string;
  items: TimelineItem[];
  onJump: (d: string) => void;
}) {
  const { i18n, t } = useTranslation();
  const total = items
    .filter((i) => !i.is_skipped && i.duration_minute != null)
    .reduce((s, i) => s + (i.duration_minute ?? 0), 0);
  return (
    <button
      onClick={() => onJump(date)}
      className="flex min-w-0 flex-1 flex-col gap-1 border-r border-border-subtle p-2 text-left"
    >
      <div className="flex items-baseline justify-between">
        <span className="font-mono text-[11px]" style={{ color: "var(--text-tertiary)" }}>
          {weekdayLabel(date, i18n.language)}
        </span>
        <span className="text-[11px]" style={{ color: "var(--text-secondary)" }}>
          {monthDay(date, i18n.language)}
        </span>
      </div>
      {items.length === 0 ? (
        <span className="text-[10px]" style={{ color: "var(--text-tertiary)" }}>
          {t("week.empty")}
        </span>
      ) : (
        <div className="flex flex-col gap-0.5">
          {items.slice(0, 5).map((it) => (
            <div
              key={it.id}
              className="truncate rounded-sm px-1 text-[10px]"
              style={{
                background: it.is_done ? "var(--signal-success-subtle)" : "var(--accent-oxide-subtle)",
                color: "var(--text-primary)",
                textDecoration: it.is_done ? "line-through" : "none",
              }}
              title={it.title}
            >
              {it.start_minute != null ? `${Math.floor(it.start_minute / 60).toString().padStart(2, "0")}:${(it.start_minute % 60).toString().padStart(2, "0")} ` : ""}
              {it.title}
            </div>
          ))}
          {items.length > 5 && (
            <span className="text-[10px]" style={{ color: "var(--text-tertiary)" }}>
              +{items.length - 5}
            </span>
          )}
        </div>
      )}
      <span className="mt-auto font-mono text-[10px]" style={{ color: "var(--text-tertiary)" }}>
        {t("week.workload", { n: total })}
      </span>
    </button>
  );
}

export function WeekView() {
  const { t } = useTranslation();
  const ui = useUi();
  const today = todayStr();
  const from = addDays(today, -3);
  const to = addDays(today, 3);
  const tlQ = useTimelineRange(from, to);

  return (
    <div className="flex h-full flex-col">
      <div className="border-b border-border-subtle px-3 py-2 text-[12px]" style={{ color: "var(--text-secondary)" }}>
        {t("week.title")}
      </div>
      <div className="flex flex-1 overflow-hidden">
        {(tlQ.data ?? []).map(({ date, items }) => (
          <MiniDay
            key={date}
            date={date}
            items={items}
            onJump={(d) => {
              ui.setDate(d);
              ui.setView("today");
            }}
          />
        ))}
      </div>
    </div>
  );
}
```

`lib/store.ts`에 `addDays(dateStr, days)`가 이미 export 되어 있는지 확인하고 없으면 추가. (기존 `shift` 함수 재활용 또는 신규 `addDays`.)

- [ ] **Step 4: `App.tsx`에서 WeekPlaceholder 교체 + 키 가드**

`crates/oxiline-app/src/App.tsx`:
- import: `import { WeekView } from "./components/WeekView";` 추가
- `WeekPlaceholder` 컴포넌트와 `{view === "week" && <WeekPlaceholder />}` 라인을 각각 `<WeekView />`으로 교체
- `useGlobalKeys`의 `←`/`→` 핸들러는 `if (view !== "today") return;`로 가드

- [ ] **Step 5: 프론트엔드 빌드 확인**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline/crates/oxiline-app
bun run build 2>&1 | tail -20
```

기대: `dist/` 갱신, 에러 없음.

- [ ] **Step 6: 커밋**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline
git add crates/oxiline-app/src/hooks.ts \
        crates/oxiline-app/src/components/WeekView.tsx \
        crates/oxiline-app/src/App.tsx \
        crates/oxiline-app/src/locales/ko.json \
        crates/oxiline-app/src/locales/en.json
git -c user.email=oxi@local -c user.name=Oxi commit -m "feat(app): week view with 7 mini timelines"
```

---

## Task 7: 드래그 앤 드롭 의존성 + DndContext 마운트

**Files:**
- Modify: `crates/oxiline-app/package.json`
- Modify: `crates/oxiline-app/src/App.tsx`

**Interfaces:**
- Consumes: 기존 `App.tsx`
- Produces: `App.tsx`가 `DndContext`로 자식들을 감쌈 (감지는 Task 8에서)

- [ ] **Step 1: dnd-kit 설치**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline/crates/oxiline-app
bun add @dnd-kit/core @dnd-kit/utilities
```

기대: `package.json`의 `dependencies`에 두 패키지 추가, `bun.lock` 갱신.

- [ ] **Step 2: `App.tsx`에 DndContext 추가**

`crates/oxiline-app/src/App.tsx`의 `App` 컴포넌트 본문 (`<div ...>`) 전체를 `<DndContext>`로 감쌈:

```tsx
import { DndContext, PointerSensor, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";

export default function App() {
  useGlobalKeys();
  const view = useUi((s) => s.view);
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 4 } }));

  return (
    <DndContext sensors={sensors}>
      <div className="flex h-screen flex-col" style={{ background: "var(--surface-canvas)" }}>
        ...
      </div>
    </DndContext>
  );
}
```

`onDragEnd`는 Task 8에서 정의. 지금은 마운트만.

- [ ] **Step 3: 빌드 확인**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline/crates/oxiline-app
bun run build 2>&1 | tail -20
```

기대: 성공.

- [ ] **Step 4: 커밋**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline
git add crates/oxiline-app/package.json crates/oxiline-app/bun.lock crates/oxiline-app/src/App.tsx
git -c user.email=oxi@local -c user.name=Oxi commit -m "feat(app): dnd-kit + DndContext mount"
```

---

## Task 8: 드래그 앤 드롭 (블록 이동/리사이즈/백로그→타임라인)

**Files:**
- Create: `crates/oxiline-app/src/lib/dnd.ts`
- Modify: `crates/oxiline-app/src/lib/api.ts`
- Modify: `crates/oxiline-app/src-tauri/src/commands.rs`
- Modify: `crates/oxiline-app/src-tauri/src/lib.rs`
- Modify: `crates/oxiline-app/src/components/BlockView.tsx`
- Modify: `crates/oxiline-app/src/components/DayTimeline.tsx`
- Modify: `crates/oxiline-app/src/components/BacklogView.tsx`

**Interfaces:**
- Consumes: `BlockView` (타임라인 블록), `BacklogView` 행
- Produces: `dnd.ts`에 `useDndHandlers()` 훅 — `DndContext`의 `onDragEnd`에서 호출. 3가지 액션: `move`, `resize`, `scheduleFromBacklog`. `api.materializeIfVirtual()` 호출.

- [ ] **Step 1: `commands.rs`에 materialize_if_virtual 커맨드 추가**

`crates/oxiline-app/src-tauri/src/commands.rs` 끝에:

```rust
#[tauri::command]
#[specta::specta]
pub fn materialize_if_virtual(
    state: State<AppState>,
    id: String,
) -> Result<String, String> {
    oxiline_core::tasks::materialize_if_virtual(&state.conn(), &id).map_err(map_err)
}
```

- [ ] **Step 2: `lib.rs`의 `collect_commands!`에 추가**

```rust
commands::materialize_if_virtual,
```

- [ ] **Step 3: `api.ts`에 메서드 추가**

```ts
materializeIfVirtual: (id: string) => invoke<string>("materialize_if_virtual", { id }),
```

- [ ] **Step 4: `lib/dnd.ts` 생성**

`crates/oxiline-app/src/lib/dnd.ts`:

```ts
import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { DragEndEvent } from "@dnd-kit/core";
import { api } from "./api";

export type DndAction =
  | { kind: "move"; id: string; startMinute: number; date: string | null }
  | { kind: "resize"; id: string; durationMinute: number }
  | { kind: "scheduleFromBacklog"; taskId: string; date: string; startMinute: number };

function snap5(min: number): number {
  return Math.max(0, Math.min(1439, Math.round(min / 5) * 5));
}

export function useDndActions() {
  const qc = useQueryClient();
  const updateTask = useMutation({
    mutationFn: (args: { id: string; patch: Record<string, unknown> }) =>
      api.updateTask(args.id, args.patch as any).then(() => qc.invalidateQueries()),
  });
  const materialize = useMutation({
    mutationFn: (id: string) => api.materializeIfVirtual(id),
  });

  return {
    move: async (rawId: string, startMinute: number, date: string | null) => {
      const id = rawId.startsWith("virtual:") ? await materialize.mutateAsync(rawId) : rawId;
      updateTask.mutate({ id, patch: { startMinute: snap5(startMinute), date } });
    },
    resize: async (rawId: string, durationMinute: number) => {
      const id = rawId.startsWith("virtual:") ? await materialize.mutateAsync(rawId) : rawId;
      updateTask.mutate({ id, patch: { durationMinute: Math.max(5, snap5(durationMinute)) } });
    },
    scheduleFromBacklog: async (taskId: string, date: string, startMinute: number) => {
      updateTask.mutate({ id: taskId, patch: { date, startMinute: snap5(startMinute), durationMinute: 30 } });
    },
  };
}
```

- [ ] **Step 5: `App.tsx`에 onDragEnd 핸들러 연결**

`crates/oxiline-app/src/App.tsx`의 `DndContext`에 `onDragEnd={handleDragEnd}` 추가. 핸들러는:

```tsx
import { useDndActions } from "./lib/dnd";

function App() {
  useGlobalKeys();
  const view = useUi((s) => s.view);
  const date = useUi((s) => s.date);
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 4 } }));
  const actions = useDndActions();

  const handleDragEnd = (e: DragEndEvent) => {
    const active = e.active;
    const over = e.over;
    if (!over) return;
    const data = active.data.current as { kind: string; [k: string]: any } | undefined;
    if (!data) return;

    if (data.kind === "block" || data.kind === "resize") {
      const startMin = data.item.start_minute ?? 0;
      const dy = e.delta.y;
      const pxPerMin = 56 / 60;
      if (data.kind === "block") {
        actions.move(data.item.id, startMin + dy / pxPerMin, null);
      } else {
        const newDur = (data.item.duration_minute ?? 30) + dy / pxPerMin;
        actions.resize(data.item.id, newDur);
      }
    } else if (data.kind === "backlog") {
      const startMin = Math.max(0, Math.min(1439, (over.rect?.top ?? 0) / (56 / 60) + 5 * 60));
      actions.scheduleFromBacklog(data.task.id, date, startMin);
    }
  };

  return (
    <DndContext sensors={sensors} onDragEnd={handleDragEnd}>
      ...
    </DndContext>
  );
}
```

(정확한 좌표 계산은 DayTimeline의 pxPerMin과 맞춰야 함. 위 스케치는 단순 평균값이며, 실제 구현은 Step 6의 `BlockView` 수정 시 `data`에 pxPerMin을 함께 넣고 거기서 계산.)

- [ ] **Step 6: `BlockView.tsx`에 useDraggable 통합**

`crates/oxiline-app/src/components/BlockView.tsx`의 컴포넌트 본문:

```tsx
import { useDraggable } from "@dnd-kit/core";
import { CSS } from "@dnd-kit/utilities";

// 컴포넌트 안:
const { attributes, listeners, setNodeRef, transform } = useDraggable({
  id: item.id,
  data: { kind: "block", item, pxPerMin },
});

const style = {
  transform: CSS.Translate.toString(transform),
  transition: transform ? undefined : "opacity var(--motion-sweep) var(--ease-standard), filter var(--motion-sweep) var(--ease-standard)",
  // ... 기존 style
};

return (
  <div ref={setNodeRef} style={style} {...attributes} {...listeners}>
    {/* 기존 button + children */}
  </div>
);
```

리스너 충돌 방지를 위해 `button`의 `onClick`이 dnd-kit의 `listeners.onPointerDown`과 충돌할 수 있음 → `button`에서 `onClick`은 유지, dnd는 `listeners`를 `button`이 아닌 외부 `<div>`에 부착. 또는 `button`은 dnd listeners 없이, 블록 자체에 listeners. 사용성 검증은 사용자에게 위임.

- [ ] **Step 7: `BacklogView.tsx`에 useDraggable 추가**

각 행을 `useDraggable({ id: task.id, data: { kind: "backlog", task } })`로 감싸기. 기존 `<div>`에 `setNodeRef` 부착.

- [ ] **Step 8: `DayTimeline.tsx`에 useDroppable + 키보드 대안**

빈 슬롯 영역 (`<div className="absolute left-0 right-0" onClick={...}>`)을 `useDroppable({ id: "slot-empty", data: { kind: "slot", date, pxPerMin } })`로 변환. 기존 onClick은 dnd와 충돌하지 않게 `onDoubleClick`으로 변경 (인라인 추가는 더블클릭).

키보드 대안: `useGlobalKeys`에 추가:

```ts
if (focusInsideBlock) {
  if (e.altKey && e.key === "ArrowUp") { /* 5분 위로 */ }
  if (e.altKey && e.key === "ArrowDown") { /* 5분 아래로 */ }
  if (e.metaKey && e.key === "ArrowUp") { /* 5분 줄이기 */ }
  if (e.metaKey && e.key === "ArrowDown") { /* 5분 늘리기 */ }
}
```

실제 키 처리는 BlockView에 `tabIndex={0}` 부여 + `onKeyDown` 핸들러 부착으로 옮기는 게 더 깔끔. Task 8은 dnd 골격만, 키보드 디테일은 컴포넌트 PR에서 마무리.

- [ ] **Step 9: 빌드 확인**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline
cargo build -p oxiline-app 2>&1 | tail -10
cd crates/oxiline-app && bun run build 2>&1 | tail -10
```

두 명령 모두 `Finished`/`dist` 갱신.

- [ ] **Step 10: 커밋**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline
git add crates/oxiline-app/src/lib/dnd.ts \
        crates/oxiline-app/src/lib/api.ts \
        crates/oxiline-app/src-tauri/src/commands.rs \
        crates/oxiline-app/src-tauri/src/lib.rs \
        crates/oxiline-app/src/components/BlockView.tsx \
        crates/oxiline-app/src/components/DayTimeline.tsx \
        crates/oxiline-app/src/components/BacklogView.tsx \
        crates/oxiline-app/src/App.tsx
git -c user.email=oxi@local -c user.name=Oxi commit -m "feat(app): drag-and-drop for blocks, resize, and backlog scheduling"
```

---

## Task 9: 루틴 그룹 CRUD (코어 + Tauri + CLI)

**Files:**
- Modify: `crates/oxiline-core/src/routines.rs`
- Modify: `crates/oxiline-core/src/lib.rs`
- Modify: `crates/oxiline-app/src-tauri/src/commands.rs`
- Modify: `crates/oxiline-app/src-tauri/src/lib.rs`
- Modify: `crates/oxiline-cli/src/cli.rs`
- Modify: `crates/oxiline-cli/src/main.rs`

**Interfaces:**
- Consumes: 기존 `routines::*` 함수
- Produces: `routines::groups::list/get/create/update/delete/set_active`. Tauri 5 커맨드. CLI `routine group add/list/show/edit/rm/toggle` 서브커맨드.

- [ ] **Step 1: `routines.rs`에 `groups` 모듈 추가**

`crates/oxiline-core/src/routines.rs` 끝에:

```rust
pub mod groups {
    use super::*;
    use crate::model::RoutineGroup;

    pub fn row_from(row: &rusqlite::Row<'_>) -> rusqlite::Result<RoutineGroup> {
        Ok(RoutineGroup {
            id: row.get(0)?,
            name: row.get(1)?,
            icon: row.get(2)?,
            is_active: row.get::<_, i64>(3)? != 0,
            sort_order: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    }

    pub fn list(conn: &Connection) -> Result<Vec<RoutineGroup>> {
        let mut stmt = conn.prepare("SELECT id, name, icon, is_active, sort_order, created_at, updated_at FROM routine_groups ORDER BY sort_order, name")?;
        let rows = stmt.query_map([], row_from)?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    pub fn get(conn: &Connection, id: &str) -> Result<RoutineGroup> {
        conn.query_row(
            "SELECT id, name, icon, is_active, sort_order, created_at, updated_at FROM routine_groups WHERE id = ?",
            params![id], row_from,
        ).map_err(CoreError::from)
    }

    pub fn create(conn: &Connection, name: &str, icon: Option<&str>) -> Result<RoutineGroup> {
        let id = crate::util::new_id();
        let now = crate::util::now_iso();
        let sort_order: i64 = conn.query_row(
            "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM routine_groups",
            [], |r| r.get(0),
        )?;
        conn.execute(
            "INSERT INTO routine_groups (id, name, icon, is_active, sort_order, created_at, updated_at) VALUES (?, ?, ?, 1, ?, ?, ?)",
            params![id, name, icon, sort_order, now, now],
        )?;
        get(conn, &id)
    }

    pub fn update(conn: &Connection, id: &str, name: Option<&str>, icon: Option<Option<&str>>, sort_order: Option<i64>) -> Result<RoutineGroup> {
        let mut sets: Vec<&str> = Vec::new();
        let mut binds: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(n) = name { sets.push("name = ?"); binds.push(Box::new(n.to_string())); }
        if let Some(ic) = icon { sets.push("icon = ?"); binds.push(Box::new(ic.map(|s| s.to_string()).unwrap_or_default())); }
        if let Some(so) = sort_order { sets.push("sort_order = ?"); binds.push(Box::new(so)); }
        sets.push("updated_at = ?");
        binds.push(Box::new(crate::util::now_iso()));
        if sets.is_empty() { return get(conn, id); }
        let sql = format!("UPDATE routine_groups SET {} WHERE id = ?", sets.join(", "));
        let mut params_vec: Vec<&dyn rusqlite::ToSql> = binds.iter().map(|b| &**b as &dyn rusqlite::ToSql).collect();
        params_vec.push(&id);
        conn.execute(&sql, params_vec.as_slice())?;
        get(conn, id)
    }

    pub fn delete(conn: &Connection, id: &str) -> Result<()> {
        let n = conn.execute("DELETE FROM routine_groups WHERE id = ?", params![id])?;
        if n == 0 { return Err(CoreError::NotFound(format!("group {id}"))); }
        Ok(())
    }

    /// Set group's is_active AND propagate to all child blocks (idempotent).
    /// Returns the updated list of child blocks.
    pub fn set_active(conn: &Connection, id: &str, active: bool) -> Result<Vec<RoutineBlock>> {
        let tx = conn.unchecked_transaction()?;
        let n = tx.execute("UPDATE routine_groups SET is_active = ?, updated_at = ? WHERE id = ?", params![active as i64, crate::util::now_iso(), id])?;
        if n == 0 { return Err(CoreError::NotFound(format!("group {id}"))); }
        tx.execute("UPDATE routine_blocks SET is_active = ?, updated_at = ? WHERE group_id = ?", params![active as i64, crate::util::now_iso(), id])?;
        let mut stmt = tx.prepare("SELECT * FROM routine_blocks WHERE group_id = ?")?;
        let rows = stmt.query_map(params![id], super::row_from)?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        tx.commit()?;
        Ok(out)
    }
}
```

- [ ] **Step 2: Tauri 5 커맨드 추가**

`crates/oxiline-app/src-tauri/src/commands.rs`에:

```rust
#[tauri::command]
#[specta::specta]
pub fn list_routine_groups(state: State<AppState>) -> Result<Vec<oxiline_core::model::RoutineGroup>, String> {
    oxiline_core::routines::groups::list(&state.conn()).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn create_routine_group(
    state: State<AppState>,
    name: String,
    icon: Option<String>,
) -> Result<oxiline_core::model::RoutineGroup, String> {
    oxiline_core::routines::groups::create(&state.conn(), &name, icon.as_deref()).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn update_routine_group(
    state: State<AppState>,
    id: String,
    name: Option<String>,
    icon: Option<Option<String>>,
    sort_order: Option<i64>,
) -> Result<oxiline_core::model::RoutineGroup, String> {
    let icon_ref = icon.as_ref().map(|opt| opt.as_deref());
    oxiline_core::routines::groups::update(&state.conn(), &id, name.as_deref(), icon_ref, sort_order).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn delete_routine_group(state: State<AppState>, id: String) -> Result<(), String> {
    oxiline_core::routines::groups::delete(&state.conn(), &id).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn set_routine_group_active(
    state: State<AppState>,
    id: String,
    active: bool,
) -> Result<Vec<RoutineBlock>, String> {
    oxiline_core::routines::groups::set_active(&state.conn(), &id, active).map_err(map_err)
}
```

- [ ] **Step 3: `lib.rs`의 `collect_commands!`에 추가**

```rust
commands::list_routine_groups,
commands::create_routine_group,
commands::update_routine_group,
commands::delete_routine_group,
commands::set_routine_group_active,
```

- [ ] **Step 4: CLI에 `routine group` 서브커맨드 추가**

`crates/oxiline-cli/src/cli.rs`의 `RoutineAction` enum에 `Group(GroupAction)` variant 추가. `GroupAction` enum:

```rust
#[derive(Subcommand)]
pub enum GroupAction {
    Add { name: String, #[arg(long)] icon: Option<String> },
    List,
    Show { id: String },
    Edit {
        id: String,
        #[arg(long)] name: Option<String>,
        #[arg(long)] icon: Option<String>,
        #[arg(long, value_parser = clap::value_parser!(i64))] sort_order: Option<i64>,
    },
    Rm { id: String },
    Toggle { id: String, #[arg(long, conflicts_with = "off")] on: bool, #[arg(long, conflicts_with = "on")] off: bool },
}
```

`main.rs`의 `Command::Routine(act)` match에 `RoutineAction::Group(g)` 분기 추가. 각 핸들러는 `routines::groups::*`을 호출하고 `output::routine_text` 등을 적절히 사용 (그룹 전용 출력은 단순 JSON dump).

- [ ] **Step 5: 빌드 + CLI 스모크**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline
cargo build --workspace 2>&1 | tail -20
OXILINE_DB_PATH=/tmp/oxiline-grp.db rm -f /tmp/oxiline-grp.db /tmp/oxiline-grp.db-*
OXILINE_DB_PATH=/tmp/oxiline-grp.db /Volumes/MERCURY/PROJECTS/oxiline/target/debug/oxiline routine group add "평일 아침" --icon sun
OXILINE_DB_PATH=/tmp/oxiline-grp.db /Volumes/MERCURY/PROJECTS/oxiline/target/debug/oxiline routine group list --json
```

기대: 첫 번째로 id 출력, 두 번째로 JSON 배열 (1개). 임시 DB 정리.

- [ ] **Step 6: 코어 통합 테스트 회귀 확인**

```bash
cargo test -p oxiline-core --tests
```

기대: 12/12 통과 (기존 동작에 영향 없음, 새 함수는 별도 라운드에서 테스트).

- [ ] **Step 7: 커밋**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline
git add crates/oxiline-core/src/routines.rs \
        crates/oxiline-core/src/lib.rs \
        crates/oxiline-app/src-tauri/src/commands.rs \
        crates/oxiline-app/src-tauri/src/lib.rs \
        crates/oxiline-cli/src/cli.rs \
        crates/oxiline-cli/src/main.rs
git -c user.email=oxi@local -c user.name=Oxi commit -m "feat(core+app+cli): routine group CRUD"
```

---

## Task 10: RoutineManager 인라인 편집 + 그룹 UI

**Files:**
- Modify: `crates/oxiline-app/src/lib/api.ts`
- Modify: `crates/oxiline-app/src/hooks.ts`
- Modify: `crates/oxiline-app/src/components/RoutineManager.tsx`
- Modify: `crates/oxiline-app/src/locales/ko.json`
- Modify: `crates/oxiline-app/src/locales/en.json`

**Interfaces:**
- Consumes: Task 9의 `list_routine_groups` 등, 기존 `RoutineManager.tsx`
- Produces: 좌측 그룹 사이드바 + 우측 블록 리스트(인라인 폼 펼침) UI

- [ ] **Step 1: i18n 키 추가**

`ko.json`에:

```json
"routineGroup": {
  "sidebar": "그룹",
  "new": "+ 새 그룹",
  "allGroups": "전체",
  "selectGroup": "그룹을 선택하세요",
  "name": "이름",
  "icon": "아이콘"
},
"routineEdit": {
  "title": "제목",
  "start": "시작",
  "duration": "소요 (분)",
  "days": "요일",
  "weekdays": "평일",
  "weekends": "주말",
  "daily": "매일",
  "category": "카테고리",
  "notes": "메모",
  "save": "저장",
  "cancel": "취소",
  "delete": "삭제"
}
```

`en.json`에 동등.

- [ ] **Step 2: `api.ts`에 5개 메서드 추가**

```ts
listRoutineGroups: () => invoke<RoutineGroup[]>("list_routine_groups"),
createRoutineGroup: (name: string, icon: string | null) =>
  invoke<RoutineGroup>("create_routine_group", { name, icon }),
updateRoutineGroup: (id: string, patch: { name?: string; icon?: string | null; sortOrder?: number }) =>
  invoke<RoutineGroup>("update_routine_group", { id, ...patch }),
deleteRoutineGroup: (id: string) => invoke<void>("delete_routine_group", { id }),
setRoutineGroupActive: (id: string, active: boolean) =>
  invoke<RoutineBlock[]>("set_routine_group_active", { id, active }),
```

`types.ts`에 `RoutineGroup` 인터페이스 추가:

```ts
export interface RoutineGroup {
  id: string;
  name: string;
  icon: string | null;
  is_active: boolean;
  sort_order: number;
}
```

- [ ] **Step 3: `hooks.ts`에 5개 훅 추가**

기존 `useInvalidate()` (private, `qc.invalidateQueries()` 전체 무효화) 패턴을 따른다. `useSetRoutineGroupActive`는 `["routine-groups"]` + `["routines"]` 두 쿼리 키를 무효화해야 하므로 그 훅만 `useQueryClient`를 직접 사용:

```ts
// hooks.ts 안, useRoutineGroups는 일반 useQuery
export function useRoutineGroups() {
  return useQuery({ queryKey: ["routine-groups"], queryFn: api.listRoutineGroups });
}

export function useCreateRoutineGroup() {
  const inv = useInvalidate();
  return useMutation({
    mutationFn: (args: { name: string; icon: string | null }) => api.createRoutineGroup(args.name, args.icon),
    onSuccess: () => inv(),
  });
}

export function useUpdateRoutineGroup() {
  const inv = useInvalidate();
  return useMutation({
    mutationFn: (args: { id: string; patch: { name?: string; icon?: string | null; sortOrder?: number } }) =>
      api.updateRoutineGroup(args.id, args.patch),
    onSuccess: () => inv(),
  });
}

export function useDeleteRoutineGroup() {
  const inv = useInvalidate();
  return useMutation({
    mutationFn: (id: string) => api.deleteRoutineGroup(id),
    onSuccess: () => inv(),
  });
}

export function useSetRoutineGroupActive() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (args: { id: string; active: boolean }) => api.setRoutineGroupActive(args.id, args.active),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["routine-groups"] });
      qc.invalidateQueries({ queryKey: ["routines"] });
    },
  });
}
```

(`useInvalidate`는 hooks.ts에 이미 정의된 private helper. `useQueryClient`는 파일 상단에서 이미 import 되어 있음.)

- [ ] **Step 4: `RoutineManager.tsx` 재작성**

기존 파일 본문을 다음 골격으로 교체. 핵심은 좌측 그룹 사이드바 + 우측 블록 리스트(클릭 → 인라인 폼):

```tsx
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useCategories, useRoutines, useCreateRoutine, useUpdateRoutine, useDeleteRoutine, useRoutineGroups, useCreateRoutineGroup, useUpdateRoutineGroup, useDeleteRoutineGroup, useSetRoutineGroupActive } from "../hooks";
import type { RoutineBlock, RoutineGroup } from "../types";

export function RoutineManager({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { t } = useTranslation();
  const groupsQ = useRoutineGroups();
  const createGroup = useCreateRoutineGroup();
  const updateGroup = useUpdateRoutineGroup();
  const deleteGroup = useDeleteRoutineGroup();
  const setGroupActive = useSetRoutineGroupActive();
  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(null);
  const routinesQ = useRoutines(false);
  const catsQ = useCategories();
  const createRoutine = useCreateRoutine();
  const updateRoutine = useUpdateRoutine();
  const deleteRoutine = useDeleteRoutine();
  const [editingId, setEditingId] = useState<string | null>(null);

  if (!open) return null;

  const selectedGroup: RoutineGroup | null =
    selectedGroupId === null
      ? null
      : groupsQ.data?.find((g) => g.id === selectedGroupId) ?? null;

  const visibleRoutines = (routinesQ.data ?? []).filter((r) =>
    selectedGroupId === null ? true : r.group_id === selectedGroupId,
  );

  return (
    <div className="fixed inset-0 z-50 flex" role="dialog" aria-label={t("routineGroup.sidebar")}>
      <aside className="flex w-48 flex-col border-r border-border-subtle bg-raised p-2">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-[12px] font-medium" style={{ color: "var(--text-secondary)" }}>{t("routineGroup.sidebar")}</h3>
          <button
            aria-label={t("routineGroup.new")}
            onClick={() => {
              const name = window.prompt(t("routineGroup.name"));
              if (name) createGroup.mutate({ name, icon: null });
            }}
          >+</button>
        </div>
        <ul className="flex-1 overflow-y-auto">
          {groupsQ.data?.map((g) => (
            <li key={g.id}>
              <button
                onClick={() => setSelectedGroupId(g.id)}
                aria-current={selectedGroupId === g.id}
                className="flex w-full items-center justify-between rounded-sm px-2 py-1 text-left text-[12px]"
                style={{ background: selectedGroupId === g.id ? "var(--accent-oxide-subtle)" : "transparent" }}
              >
                <span>{g.name}</span>
                <input
                  type="checkbox"
                  checked={g.is_active}
                  onChange={(e) => setGroupActive.mutate({ id: g.id, active: e.target.checked })}
                  onClick={(e) => e.stopPropagation()}
                  aria-label={g.name}
                />
              </button>
            </li>
          ))}
        </ul>
        <button onClick={onClose} className="text-[11px]" style={{ color: "var(--text-tertiary)" }}>닫기</button>
      </aside>
      <main className="flex-1 overflow-y-auto p-3">
        {!selectedGroup ? (
          <p style={{ color: "var(--text-tertiary)" }}>{t("routineGroup.selectGroup")}</p>
        ) : (
          <>
            <h3 className="mb-2 text-[14px] font-medium">{selectedGroup.name}</h3>
            {visibleRoutines.map((r) =>
              editingId === r.id ? (
                <InlineRoutineForm
                  key={r.id}
                  routine={r}
                  categories={catsQ.data ?? []}
                  onSave={(patch) => {
                    updateRoutine.mutate({ id: r.id, patch });
                    setEditingId(null);
                  }}
                  onCancel={() => setEditingId(null)}
                  onDelete={() => {
                    deleteRoutine.mutate(r.id);
                    setEditingId(null);
                  }}
                />
              ) : (
                <button
                  key={r.id}
                  onClick={() => setEditingId(r.id)}
                  className="mb-1 flex w-full items-center justify-between rounded-md border border-border-subtle bg-raised px-2 py-1.5 text-left"
                >
                  <span className="truncate text-[13px]">{r.title}</span>
                  <span className="font-mono text-[11px]" style={{ color: "var(--text-tertiary)" }}>
                    {String(Math.floor(r.start_minute / 60)).padStart(2, "0")}:{String(r.start_minute % 60).padStart(2, "0")} · {r.duration_minute}분
                  </span>
                </button>
              ),
            )}
            <button
              onClick={() => createRoutine.mutate({
                title: "새 루틴", startMinute: 9 * 60, durationMinute: 30, weekdayMask: 31,
                categoryId: null, effectiveFrom: null, effectiveUntil: null, notes: null,
              } as any)}
              className="mt-2 text-[12px]"
              style={{ color: "var(--accent-oxide-strong)" }}
            >+ {t("routineEdit.title")}</button>
          </>
        )}
      </main>
    </div>
  );
}

function InlineRoutineForm({ routine, categories, onSave, onCancel, onDelete }: {
  routine: RoutineBlock;
  categories: { id: string; name: string }[];
  onSave: (patch: Partial<RoutineBlock>) => void;
  onCancel: () => void;
  onDelete: () => void;
}) {
  const { t } = useTranslation();
  const [title, setTitle] = useState(routine.title);
  const [startMinute, setStartMinute] = useState(routine.start_minute);
  const [durationMinute, setDurationMinute] = useState(routine.duration_minute);
  const [weekdayMask, setWeekdayMask] = useState(routine.weekday_mask);
  const [categoryId, setCategoryId] = useState<string | null>(routine.category_id);
  const [notes, setNotes] = useState(routine.notes ?? "");

  return (
    <div className="mb-2 flex flex-col gap-2 rounded-md border p-2" style={{ borderColor: "var(--accent-oxide)" }}>
      <input
        aria-label={t("routineEdit.title")}
        value={title}
        onChange={(e) => setTitle(e.target.value)}
        className="rounded-sm border border-border-subtle bg-transparent px-1 text-[13px]"
      />
      <div className="flex gap-2">
        <label className="text-[11px]">
          {t("routineEdit.start")}
          <input
            type="time"
            value={`${String(Math.floor(startMinute / 60)).padStart(2, "0")}:${String(startMinute % 60).padStart(2, "0")}`}
            onChange={(e) => {
              const [h, m] = e.target.value.split(":").map(Number);
              setStartMinute(h * 60 + m);
            }}
            className="ml-1 font-mono text-[12px]"
          />
        </label>
        <label className="text-[11px]">
          {t("routineEdit.duration")}
          <input
            type="number"
            min={5}
            step={5}
            value={durationMinute}
            onChange={(e) => setDurationMinute(Number(e.target.value))}
            className="ml-1 w-16 text-[12px]"
          />
        </label>
      </div>
      <div className="flex gap-1">
        {(["mon", "tue", "wed", "thu", "fri", "sat", "sun"] as const).map((d, i) => (
          <button
            key={d}
            type="button"
            aria-pressed={(weekdayMask >> i) & 1}
            onClick={() => setWeekdayMask(weekdayMask ^ (1 << i))}
            className="rounded-sm border px-1.5 py-0.5 text-[10px]"
            style={{ background: (weekdayMask >> i) & 1 ? "var(--accent-oxide)" : "transparent", color: (weekdayMask >> i) & 1 ? "white" : "var(--text-secondary)" }}
          >{d}</button>
        ))}
      </div>
      <select
        aria-label={t("routineEdit.category")}
        value={categoryId ?? ""}
        onChange={(e) => setCategoryId(e.target.value || null)}
        className="rounded-sm border border-border-subtle bg-transparent px-1 text-[12px]"
      >
        <option value="">—</option>
        {categories.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}
      </select>
      <textarea
        aria-label={t("routineEdit.notes")}
        value={notes}
        onChange={(e) => setNotes(e.target.value)}
        rows={2}
        className="rounded-sm border border-border-subtle bg-transparent px-1 text-[12px]"
      />
      <div className="flex gap-1">
        <button
          onClick={() => onSave({ title, startMinute, durationMinute, weekdayMask, categoryId, notes: notes || null })}
          className="rounded-sm px-2 py-0.5 text-[12px]"
          style={{ background: "var(--accent-oxide)", color: "white" }}
        >{t("routineEdit.save")}</button>
        <button onClick={onCancel} className="rounded-sm border px-2 py-0.5 text-[12px]">{t("routineEdit.cancel")}</button>
        <button onClick={onDelete} className="ml-auto rounded-sm px-2 py-0.5 text-[12px]" style={{ color: "var(--signal-rust)" }}>{t("routineEdit.delete")}</button>
      </div>
    </div>
  );
}
```

(이 코드는 골격이며, 실제 PR에서 미세 조정 — `useSetRoutineGroupActive` 훅의 qc import 누락 등.)

- [ ] **Step 5: 빌드 확인**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline
cargo build -p oxiline-app 2>&1 | tail -10
cd crates/oxiline-app && bun run build 2>&1 | tail -10
```

두 명령 모두 통과.

- [ ] **Step 6: 커밋**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline
git add crates/oxiline-app/src/lib/api.ts \
        crates/oxiline-app/src/hooks.ts \
        crates/oxiline-app/src/types.ts \
        crates/oxiline-app/src/components/RoutineManager.tsx \
        crates/oxiline-app/src/locales/ko.json \
        crates/oxiline-app/src/locales/en.json
git -c user.email=oxi@local -c user.name=Oxi commit -m "feat(app): routine manager with group sidebar + inline editor"
```

---

## Task 11: 접근성 마무리 (aria-label, focus, role)

**Files:**
- Modify: `crates/oxiline-app/src/components/Header.tsx`
- Modify: `crates/oxiline-app/src/components/DayTimeline.tsx`
- Modify: `crates/oxiline-app/src/components/BacklogView.tsx`
- Modify: `crates/oxiline-app/src/components/CommandPalette.tsx`
- Modify: `crates/oxiline-app/src/components/Preferences.tsx`
- Modify: `crates/oxiline-app/src/locales/ko.json`
- Modify: `crates/oxiline-app/src/locales/en.json`

**Interfaces:**
- Consumes: 기존 컴포넌트, i18n 리소스
- Produces: 모든 인터랙티브 요소에 `aria-label`, 토글/체크박스에 `role` + `aria-checked`/`aria-pressed`, 포커스 가능한 dnd 요소에 `tabIndex=0`

- [ ] **Step 1: i18n 키 추가 (a11y 공통)**

`ko.json` `common`에:

```json
"a11y": {
  "previousDay": "이전 날",
  "nextDay": "다음 날",
  "today": "오늘로",
  "openCommandPalette": "커맨드 팔레트 열기",
  "openPreferences": "환경설정 열기",
  "openRoutineManager": "루틴 관리 열기",
  "toggleDone": "완료 토글",
  "deleteTask": "할일 삭제",
  "close": "닫기",
  "moreActions": "더보기"
}
```

`en.json`에 동등.

- [ ] **Step 2: `Header.tsx` 아이콘 버튼에 aria-label**

모든 `<button>` 자식이 lucide 아이콘만 있는 경우 `<button aria-label={t("a11y.openCommandPalette")}>` 형태로 감쌈.

- [ ] **Step 3: `BlockView.tsx`에 role="button", aria-label, focusable**

`<div ref={setNodeRef} ...>`에:
- `role="button"`
- `tabIndex={0}`
- `aria-label={`${item.title}, ${rangeLabel(item.start_minute, item.duration_minute)}`}`
- `aria-pressed={item.is_done}`

- [ ] **Step 4: `BacklogView.tsx` 행에 aria-label**

각 행에 `aria-label={task.title}`.

- [ ] **Step 5: `DayTimeline.tsx` 빈 슬롯에 aria-label**

`<div onDoubleClick={...}>`에 `aria-label={t("timeline.emptySlot")}` (i18n 키 추가: `timeline.emptySlot: "빈 시간대 — 더블클릭으로 항목 추가"`).

- [ ] **Step 6: `CommandPalette.tsx` input에 aria-label, list에 role="listbox"**

input에 `aria-label={t("a11y.openCommandPalette")}`. 결과 리스트 `<ul role="listbox">`, 각 `<li role="option" aria-selected={focused === i}>`.

- [ ] **Step 7: `Preferences.tsx` 모든 토글에 role="switch" + aria-checked**

체크박스 input은 `<input role="switch" aria-checked={checked} aria-label={t("notifications.enable")} />`.

- [ ] **Step 8: 빌드 확인 + 커밋**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline/crates/oxiline-app
bun run build
cd /Volumes/MERCURY/PROJECTS/oxiline
git add crates/oxiline-app/src/components/Header.tsx \
        crates/oxiline-app/src/components/DayTimeline.tsx \
        crates/oxiline-app/src/components/BacklogView.tsx \
        crates/oxiline-app/src/components/CommandPalette.tsx \
        crates/oxiline-app/src/components/Preferences.tsx \
        crates/oxiline-app/src/components/BlockView.tsx \
        crates/oxiline-app/src/locales/ko.json \
        crates/oxiline-app/src/locales/en.json
git -c user.email=oxi@local -c user.name=Oxi commit -m "feat(app): a11y pass — aria labels, roles, focusable dnd"
```

---

## Task 12: 최종 검증

**Files:** (없음)

**Interfaces:** (없음)

- [ ] **Step 1: 전체 워크스페이스 빌드**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline
cargo build --workspace 2>&1 | tail -10
```

기대: `Finished` 라인. 경고는 무시.

- [ ] **Step 2: 코어 테스트 회귀**

```bash
cargo test -p oxiline-core --tests
```

기대: 12/12 통과.

- [ ] **Step 3: 프론트엔드 빌드**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline/crates/oxiline-app
bun run build
```

기대: 성공.

- [ ] **Step 4: CLI 새 기능 스모크**

```bash
cd /tmp
rm -f /tmp/oxiline-final.db /tmp/oxiline-final.db-*
OXILINE_DB_PATH=/tmp/oxiline-final.db /Volumes/MERCURY/PROJECTS/oxiline/target/debug/oxiline doctor
OXILINE_DB_PATH=/tmp/oxiline-final.db /Volumes/MERCURY/PROJECTS/oxiline/target/debug/oxiline routine group add "테스트" --icon sun
OXILINE_DB_PATH=/tmp/oxiline-final.db /Volumes/MERCURY/PROJECTS/oxiline/target/debug/oxiline routine group list --json
OXILINE_DB_PATH=/tmp/oxiline-final.db /Volumes/MERCURY/PROJECTS/oxiline/target/debug/oxiline settings get --json | grep -E '(notifications|lead)'
rm -f /tmp/oxiline-final.db /tmp/oxiline-final.db-*
```

기대: doctor가 `Schema version: 2`, 그룹이 추가되고 목록에 보이며, settings에 `notifications_enabled`/`notification_lead_minutes` 키가 존재.

- [ ] **Step 5: git 상태 확인**

```bash
cd /Volumes/MERCURY/PROJECTS/oxiline
git status
git log --oneline -15
```

기대: `working tree clean`, 커밋 12개가 새로 추가됨.

- [ ] **Step 6: 커밋 (변경 없으면 생략)**

변경이 있으면 (예: 설정/주석) `git add ...` 후 `chore(phase2): final tweaks` 커밋. 변경 없으면 그대로 종료.

---

## 완료

12개 작업이 모두 끝나면:
- macOS GUI 부팅 → 온보딩 (있으면) → 메인 창에서 모든 Phase 2 기능 노출
- `1`/`2`/`3` 키로 뷰 전환
- 백로그/타임라인 자유 드래그
- `⌘,`로 환경설정 → 알림·그룹 관리
- 메뉴바 아이콘에 진행률
- 전역 단축키로 HUD (NSPanel이 macOS에서 non-activating)
- `oxiline routine group ...` 등 새 CLI 명령 사용 가능

**남은 작업 (다음 라운드):**
- 통합 테스트 추가 (Phase 2의 새 함수들 + UI는 vitest/react-testing-library)
- `cargo clippy --workspace --all-targets -- -D warnings` 통과
- macOS 실기기 검증 (HUD non-activation, 트레이 진행률, 알림 발송)
- 코드 서명/공증 (Phase 3)
