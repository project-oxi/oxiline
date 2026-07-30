# 04. Architecture — Rust Workspace, Tauri 앱 구조, 백그라운드 상주와 플로팅 HUD

## 4.1 기술 스택 확정 (2026년 중반 기준 최신)

| 영역 | 선택 | 비고 |
|---|---|---|
| 앱 프레임워크 | **Tauri v2** (2.11.x 이상, 2026년 7월 기준 최신 안정 라인) | 구현 시점에 `cargo add tauri@latest`로 최신 패치 확인 |
| 언어 | Rust (edition **2024**, 2025년 stable 리버스 정착) | 최신 stable 툴체인 사용, MSRV 고정하지 않음(단일 개발자 로컬 빌드) |
| 프론트엔드 | React 19 + TypeScript + Vite | Tauri 공식 템플릿 기준 |
| 스타일링 | Tailwind CSS v4 (OKLCH 네이티브 지원) | `06-design-system.md`의 토큰을 CSS 커스텀 프로퍼티로 등록 후 Tailwind에서 참조 |
| IPC 타입 안전성 | **tauri-specta** (v2) | Rust 커맨드/이벤트 시그니처에서 TypeScript 바인딩 자동 생성 — 수동 타입 동기화 제거 |
| DB | SQLite + **rusqlite** (bundled feature) | 비동기 불필요(로컬 단일 사용자), WAL 모드 (`03-data-model.md` §3.9) |
| DB 커넥션 관리 | GUI: `r2d2` + `r2d2_sqlite` 풀 / CLI: 단발성 `Connection::open` | CLI는 실행마다 열고 닫는 짧은 프로세스이므로 풀 불필요 |
| 마이그레이션 | `rusqlite_migration` | `oxiline-core`에만 존재 |
| CLI 파서 | `clap` v4 (derive) | `06-cli-spec.md` |
| 전역 단축키 | `tauri-plugin-global-shortcut` | |
| 트레이 아이콘 | Tauri 내장 `tray-icon` feature | |
| 플로팅 HUD | `tauri-nspanel` (ahkohd/tauri-nspanel) | 진짜 non-activating NSPanel — 포커스를 뺏지 않음, 전체화면 앱 위에도 표시 |
| 자동 실행 | `tauri-plugin-autostart` | |
| 단일 인스턴스 | `tauri-plugin-single-instance` | 두 번째 실행 시 기존 창 포커스 |
| 트레이 위치 계산 | `tauri-plugin-positioner` | HUD/드롭다운을 트레이 아이콘 기준 배치 |
| 파일 변경 감지 | `notify` | CLI가 DB를 건드렸을 때 GUI가 갱신하도록 (§4.5) |
| React 측 i18n | `react-i18next` | UI 카피용 (JSON) |
| 아이콘 세트 | `lucide-react` (프론트) / lucide SVG 원본 (트레이 아이콘 렌더링) | |

> **왜 async 런타임(tokio)을 핵심에 넣지 않는가**: OxiLine의 모든 DB 작업은 로컬 파일 I/O이고
> 지연이 사실상 없다. Tauri 자체는 내부적으로 tokio를 쓰지만, `oxiline-core`의 공개 API는 **동기
> 함수**로 설계한다. 이렇게 하면 CLI(비async 바이너리로 유지 가능)와 GUI(Tauri 커맨드 내부에서
> `tauri::async_runtime::spawn_blocking`으로 감싸 호출)가 동일한 코어를 마찰 없이 재사용할 수 있다.
> 불필요한 async 전파(async 색칠 문제)를 피하는 것이 "트렌디함"보다 우선한다 — 트렌디함은 최신
> 안정적인 크레이트를 쓰는 것이지, 불필요하게 비동기화하는 것이 아니다.

## 4.2 Cargo Workspace 구조

```
oxiline/
├── Cargo.toml                  # [workspace] members
├── crates/
│   ├── oxiline-core/           # 순수 Rust 라이브러리. SQL, 도메인 로직, occurrence 병합 로직
│   │   ├── src/
│   │   │   ├── model.rs        # 도메인 타입 (03-data-model.md §3.11)
│   │   │   ├── db.rs           # 연결/마이그레이션/PRAGMA
│   │   │   ├── routines.rs     # routine_blocks CRUD
│   │   │   ├── tasks.rs        # tasks CRUD + materialize
│   │   │   ├── timeline.rs     # get_timeline_for_date() 등 병합 쿼리
│   │   │   ├── categories.rs
│   │   │   ├── settings.rs
│   │   │   └── paths.rs        # DB 파일 경로 결정 (directories crate)
│   │   └── migrations/*.sql
│   │
│   ├── oxiline-app/            # Tauri GUI 바이너리
│   │   ├── src-tauri/
│   │   │   ├── src/
│   │   │   │   ├── main.rs
│   │   │   │   ├── commands.rs # #[tauri::command] + #[specta::specta] 래퍼들
│   │   │   │   ├── hud.rs      # tauri-nspanel 기반 플로팅 패널 관리
│   │   │   │   ├── tray.rs     # 트레이 아이콘/메뉴
│   │   │   │   ├── shortcuts.rs# 전역 단축키 등록
│   │   │   │   ├── watcher.rs  # DB 파일 감시 → 프론트엔드로 이벤트 emit
│   │   │   │   └── state.rs    # AppState (DbPool 등)
│   │   │   └── tauri.conf.json
│   │   └── src/                # React 프론트엔드
│   │
│   └── oxiline-cli/            # 순수 CLI 바이너리 (Tauri 의존성 없음)
│       └── src/
│           ├── main.rs
│           ├── cli.rs           # clap derive 구조체
│           └── output.rs        # 텍스트/JSON 출력 포맷터
```

이 구조의 핵심 원칙: **`oxiline-core`는 Tauri도, clap도 모른다.** 순수하게 "OxiLine의 하루를 관리하는
라이브러리"다. `oxiline-app`과 `oxiline-cli`는 각각 얇은 어댑터일 뿐이다. 이렇게 하면:

- 두 바이너리 간 로직 drift가 구조적으로 불가능하다 (같은 함수를 호출하므로).
- `oxiline-core`에 대해 GUI/CLI 없이 순수 Rust 유닛 테스트를 작성할 수 있다 (occurrence 병합 로직처럼
  까다로운 부분일수록 중요).
- 향후 MCP 서버(`05-cli-spec.md` §5.6)나 다른 클라이언트를 추가해도 core는 그대로 재사용된다.

## 4.3 macOS 상주 프로세스 모델 — "닫아도 살아있다"는 것의 정확한 의미

사용자가 요청한 "앱이 닫혀 있을 때에도 전역 단축키가 동작"하는 요구사항은 기술적으로 다음을 의미한다
(이 구분을 명확히 하지 않으면 구현이 어긋나므로 명시한다):

- ❌ **완전 종료(process가 종료됨)** 상태에서 단축키가 앱을 깨우는 것이 아니다 (이건 OS 레벨 전역
  단축키 등록이 살아있는 프로세스 없이는 불가능하다 — 어떤 앱도 이렇게 동작하지 않는다).
- ✅ **메인 창을 닫아도 프로세스는 메뉴바 아이콘으로 계속 상주**하고, 그 상주 프로세스가 전역 단축키를
  듣고 있다가 플로팅 HUD를 띄운다. 이는 Raycast, Alfred, Bartender 같은 macOS 유틸리티의 표준 동작
  방식과 동일하다.
- 진짜 "종료(Quit)"는 트레이 메뉴의 **"OxiLine 종료"** 항목을 통해서만 가능하며, 이때는 단축키도
  동작하지 않는다. 이 트레이드오프를 온보딩 화면과 설정에 한 줄로 안내한다
  (예: "창을 닫아도 OxiLine은 메뉴바에서 계속 실행됩니다").
- `tauri-plugin-autostart`로 **로그인 시 자동 실행**을 기본값(ON)으로 설정해, 사용자가 재부팅해도
  다시 신경 쓸 필요가 없게 한다 (설정에서 끌 수 있음).

### 구현 요점

```jsonc
// tauri.conf.json (핵심 필드만)
{
  "app": {
    "macOSPrivateApi": true, // window-vibrancy 등에 필요
    "windows": [
      {
        "label": "main",
        "title": "OxiLine",
        "width": 420,
        "height": 720,
        "minWidth": 360,
        "minHeight": 480,
        "decorations": true,
        "titleBarStyle": "Overlay",   // 커스텀 드래그 리전 + 네이티브 트래픽 라이트
        "hiddenTitle": true,
        "visible": true
      }
    ]
  }
}
```

```rust
// oxiline-app/src-tauri/src/main.rs (스케치)
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // 이미 실행 중이면 기존 main 창을 보여주고 포커스
            if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.set_focus(); }
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent, None,
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_positioner::init())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory); // Dock 아이콘 숨김

            tray::build(app.handle())?;
            shortcuts::register_default(app.handle())?;
            hud::init_panel(app.handle())?;   // HUD 패널을 미리 생성해두고 평소엔 숨김
            watcher::spawn(app.handle())?;    // DB 파일 감시 스레드 시작
            Ok(())
        })
        .on_window_event(|window, event| {
            // "main" 창의 X 버튼 = 닫기가 아니라 숨기기 (프로세스는 계속 상주)
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(commands::handler())
        .run(tauri::generate_context!())
        .expect("error while running OxiLine");
}
```

메뉴 창을 "닫기"가 아니라 "숨기기"로 가로채는 패턴은 macOS 메뉴바 유틸리티의 표준 관용구다. 진짜
종료는 트레이 메뉴의 "종료" 항목에서 `app.exit(0)`을 명시적으로 호출할 때만 일어난다.

## 4.4 전역 단축키 → 플로팅 HUD 흐름

1. `tauri-plugin-global-shortcut`으로 기본 단축키(`CmdOrCtrl+Shift+O`, 설정에서 변경 가능)를 등록한다.
2. 핸들러가 호출되면:
   - `oxiline_core::timeline::get_now_context(now)`를 호출해 "지금 진행 중인 항목"과 "다음 항목"을
     계산한다 (이 함수는 CLI의 `oxiline now`와 완전히 동일한 로직을 공유한다 — `05-cli-spec.md` §5.2).
   - HUD 패널(`tauri-nspanel`로 미리 만들어둔 non-activating panel)에 데이터를 emit하고 `show()`한다.
   - 현재 포커스가 있는 화면(active display) 상단 중앙에 위치시킨다 (`tauri-plugin-positioner`
     또는 NSScreen 좌표 직접 계산).
   - `hud_duration_ms`(기본 2000ms) 후 자동으로 `hide()`. 타이머 도중 같은 단축키가 다시 눌리면
     타이머를 리셋한다(깜빡임 없이 자연스럽게 연장되는 느낌). ESC 키 또는 패널 바깥 클릭 시 즉시 숨김.
3. HUD 패널은 **절대 포커스를 가져가지 않는다** (`no_activate(true)`) — 사용자가 다른 앱에서 타이핑
   중이었다면 그 상태 그대로 유지되어야 한다. 이는 Spotlight/Raycast와 동일한 기대치다.

```rust
// hud.rs (스케치)
use tauri_nspanel::{PanelBuilder, PanelLevel, StyleMask};

pub fn init_panel(app: &AppHandle) -> tauri::Result<()> {
    PanelBuilder::new(app, "hud")
        .url(tauri::WebviewUrl::App("hud.html".into()))
        .no_activate(true)
        .level(PanelLevel::Floating)
        .style_mask(StyleMask::empty().hud_window())
        .corner_radius(16.0)
        .transparent(true)
        .collection_behavior(/* 모든 Space + 전체화면 앱 위에도 표시 */ Default::default())
        .build()?;
    Ok(())
}
```

HUD 창 자체의 `tauri.conf.json` 항목: `"decorations": false, "transparent": true, "alwaysOnTop": true,
"skipTaskbar": true, "visible": false, "resizable": false, "shadow": false"` (그림자는 패널
스타일마스크가 자체 처리).

## 4.5 GUI ↔ CLI 데이터 동기화 전략

GUI와 CLI는 소켓이나 IPC 프로토콜로 직접 통신하지 않는다 — **SQLite 파일 자체가 유일한 진실의
원천**이고, 동기화는 파일 변경 감지로 해결한다. 이렇게 하는 이유: (1) CLI가 GUI 없이도 완전히 독립적으로
동작해야 하고 (에이전트가 GUI를 실행하지 않은 상태에서도 CLI만으로 완결되어야 함), (2) 소켓 프로토콜을
추가하면 core crate가 통신 계층까지 알아야 해서 §4.2의 원칙이 깨진다.

1. GUI 시작 시 `notify` 크레이트로 DB 파일(및 WAL 파일)에 대한 파일시스템 watcher를 백그라운드
   스레드에서 띄운다.
2. CLI가 어떤 명령이든 실행해 DB에 쓰기를 하면(WAL 체크포인트/파일 변경), watcher가 이를 감지한다
   (디바운스 150ms 정도로 묶어서 과도한 리렌더 방지).
3. 변경이 감지되면 Tauri 이벤트(`"oxiline://db-changed"`)를 프론트엔드로 emit한다.
4. React 쪽에서는 해당 이벤트를 구독해 현재 보고 있는 날짜의 타임라인을 다시 fetch한다
   (React Query의 `invalidateQueries` 패턴을 권장 — 아래 §4.6 참고).

이 방식은 지연이 사실상 100ms 이내로 사용자가 "실시간"으로 느끼기에 충분하고, 별도의 IPC 프로토콜
설계/유지 비용이 없다.

## 4.6 프론트엔드 상태 관리

- **React Query(TanStack Query)**를 데이터 페칭/캐싱 레이어로 사용한다. 모든 Tauri 커맨드 호출을
  Query/Mutation으로 감싸고, §4.5의 `db-changed` 이벤트 리스너가 관련 쿼리 키를 invalidate한다.
- **Zustand**로 순수 UI 상태(선택된 날짜, 열린 모달, 커맨드 팔레트 표시 여부 등 서버 상태가 아닌 것)를
  관리한다. Redux 같은 무거운 상태 관리는 이 앱 규모에 과하다.
- tauri-specta가 생성한 `bindings.ts`를 통해 모든 Rust 커맨드 호출부가 타입 안전하다 — 수동으로
  `invoke<T>("command_name", args)` 문자열을 쓰지 않는다.

## 4.7 실시간 "지금 선(Now Line)" 렌더링

- Now Line의 위치는 매 프레임 다시 계산하지 않고, `requestAnimationFrame` 루프에서 현재 시각을 읽어
  CSS `transform: translateY(px)`를 업데이트한다 (React state를 매 프레임 바꾸면 리렌더 폭탄이 되므로
  **DOM ref에 직접 style을 쓰는 imperative 업데이트**로 구현한다. React 렌더 트리는 건드리지 않는다).
- 분 단위로 "지나간 블록"의 스타일(산화 처리 - `06-design-system.md` §6.5)을 재계산하는 것은
  `setInterval(60_000)` 정도로 충분하다 (초 단위로 정밀할 필요 없음).
- 시스템이 잠자기(sleep)에서 깨어난 경우를 대비해, 창이 포커스를 받거나 `visibilitychange` 이벤트가
  발생할 때 시간을 강제로 재동기화한다.

## 4.8 파일 시스템 경로

```rust
// paths.rs
use directories::ProjectDirs;

pub fn db_path() -> PathBuf {
    if let Ok(override_path) = std::env::var("OXILINE_DB_PATH") {
        return PathBuf::from(override_path);
    }
    let dirs = ProjectDirs::from("com", "oxi", "oxiline")
        .expect("could not resolve OS directories");
    // macOS: ~/Library/Application Support/com.oxi.oxiline/
    // 사용자에게 보이는 경로는 ~/Library/Application Support/OxiLine/ 로 맞추기 위해
    // ProjectDirs 대신 아래처럼 직접 구성하는 것을 권장:
    dirs.data_dir().join("oxiline.db")
}
```

> 구현 노트: `ProjectDirs`가 만드는 경로가 `com.oxi.oxiline`처럼 역도메인 형태로 나오는 것을 원치
> 않으면, macOS 한정 앱이므로 `~/Library/Application Support/OxiLine/oxiline.db`를 `dirs::home_dir()`
> 기반으로 직접 조합해도 무방하다. 중요한 건 **GUI와 CLI가 완전히 동일한 경로 산출 함수를 core에서
> 공유**한다는 것이다. `OXILINE_DB_PATH` 환경 변수 오버라이드는 에이전트가 격리된 테스트 DB로 작업하고
> 싶을 때(CI, 샌드박스 실행) 유용하므로 반드시 지원한다.

## 4.9 macOS 권한 및 배포 참고사항

- 전역 단축키 등록은 macOS에서 손쉬운 사용(Accessibility) 권한 프롬프트를 유발할 수 있다. 권한이
  거부되어도 앱 자체(GUI 조작)는 정상 동작해야 한다 — 전역 단축키는 "편의 기능"이지 필수 기능이
  아니라는 원칙을 지킨다 (설정 화면에 권한 상태와 "시스템 설정 열기" 버튼을 둔다).
- 배포는 코드 서명 + 공증(notarization)을 전제로 한다 (Tauri CLI의 `tauri build` + Apple Developer
  ID). 이 문서 세트에서는 배포 자동화(CI/CD)까지는 다루지 않는다 — Phase 3 이후 별도 문서화 대상.
