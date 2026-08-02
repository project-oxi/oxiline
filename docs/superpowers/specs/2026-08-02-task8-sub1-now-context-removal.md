# Task 8 서브프로젝트 1 — NowContext 제거 (recording-네이티브 now/next)

> 날짜: 2026-08-02
> 범위: 레거시 철거(Task 8) 5단계 중 **1단계**. `NowContext`/`NowItem`(legacy task/routine/timeline 기반 now/next)를 recording-네이티브 `NowSummary`로 교체하고, 3개 활성 소비자(notifier/tray/CLI `now`)를 마이그레이션한 뒤 legacy 모델·명령·dead 프론트를 제거한다.
> 후속 단계(별도 spec): 2 CommandPalette 재설계 · 3 Header 마커+BlockView · 4 레거시 4뷰 제거 · 5 코어 삭제+V5.

## 0. 배경

`timeline::get_now_context`(timeline.rs:86)는 `get_timeline_for_date`(task/routine 기반 TimelineItem)에서 current/next를 도출. 녹화 레이어(plan/slot/record)가 앱의 본체가 된 지금, now/next는 **활성 녹음 + plan 슬롯**에서 나와야 한다. HUD는 이미 마이그레이션됨(Task 3); notifier/tray/CLI `now`만 남았고, 프론트 `useNowContext`/`onNowUpdate`는 dead(소비자 0).

**활성 소비자**: `notifier.rs:49`(next로 알림), `tray.rs:120`(now_summary 메뉴 라벨), `oxiline-cli main.rs:60`(`oxiline now`). **Dead**: `get_now_context` 명령(commands.rs:216) + 프론트.

## 1. 코어 — `plan::now_summary` + 신모델

신모델(`model.rs`, `NowContext`/`NowItem` 대체):
```rust
pub struct NowSummary {
    pub current: Option<NowEntry>,
    pub next: Option<NowEntry>,
}
pub struct NowEntry {
    pub id: String,                 // record.id (활성 녹음) 또는 plan_id (슬롯)
    pub title: String,              // activity.name (첫 옵션)
    pub start_minute: Option<u16>,  // 슬롯 시작; 녹음은 None
    pub starts_in_minute: Option<i64>,  // next: 시작까지 분
    pub remaining_minute: Option<i64>,  // current 슬롯: 끝까지 분 (녹음은 None — open-ended)
}
```
- legacy cruft(`is_virtual`, `category_id`, `generated_at`) 제거 — 아무도 안 씀.

코어 `plan::now_summary(conn, now_minute: u16) -> Result<NowSummary>`(plan.rs; `slots_for_date` + `record::current` 합성):
1. `today = util::today_local()`, `now = chrono::Utc::now()`.
2. **current 우선순위**: `record::current(conn, now, &today)?.active` → 활성 녹음이면 `NowEntry{id: record.id, title: activity.name, start/starts_in/remaining: None}`(open-ended라 remaining 없음). 아니면 `slots_for_date`에서 now를 포함하는 첫 슬롯(`start <= now < start+duration`) → `NowEntry{id: plan_id, title: options[0].name, start_minute, remaining: end-now}`.
3. **next**: `slots_for_date`에서 `start_minute > now`인 슬롯 중 최소 → `NowEntry{id: plan_id, title: options[0].name, start_minute, starts_in: start-now}`.
4. `record::current` 재사용 — stale open-record 정리 부수효과 + 단일 사용자 로컌 앱에 compliance 비용 미미(60s 폴링). 경량 getter는 YAGNI.

`timeline::get_now_context`(timeline.rs:85-139) + `NowContext`/`NowItem`(model.rs:122-142) **삭제**. 단 `timeline::get_timeline_for_date`/range는 Header·CommandPalette·WeekView가 쓰므로 **유지**(3단계에서 제거).

## 2. 소비자 마이그레이션

- **notifier.rs**: `get_now_context` → `plan::now_summary`. `ctx.next` → `summary.next`. id/title/starts_in 그대로. 알림·dedup 로직 유지.
- **tray.rs `now_summary`**: `get_now_context` → `plan::now_summary`. current(remaining)/next(starts_in) 분기 유지, 라벨 동일.
- **CLI `now`**(main.rs:59-66): `plan::now_summary` → `output::now_text` 재작성(`NowSummary`/`NowEntry` 기반, 라벨/문구 동일).
- **명령 제거**: `get_now_context`(commands.rs:214-218) + `lib.rs` 등록.

## 3. 프론트 제거 (dead)

- `hooks.ts`: `useNowContext`(83-85).
- `api.ts`: `getNowContext`(73).
- `api.ts`: `onNowUpdate`(184-186) + `oxiline://now` listen.
- `types.ts`: `NowContext`/`NowItem`/`NowItem` 관련 타입(92-).
- 모두 소비자 0(탐색 확인). 제거 시 미참조면 tsc가 즉시 포착.

## 4. 검증

- **코어 테스트**(`tests/plan.rs` 신규): `now_summary` — (a) 활성 녹음 있으면 current=녹음(next는 plan); (b) 녹음 없고 현재 슬롯 있으면 current=슬롯(remaining); (c) next는 now 이후 첫 슬롯(starts_in); (d) 둘 다 없으면 current/next=None.
- **CLI**: `cargo build -p oxiline-cli` + `cargo test -p oxiline-cli`(있으면).
- **app**: `cargo build -p oxiline-app` + `npx tsc --noEmit` + `npx vitest run`.
- **잔여 NowContext grep**: `NowContext|NowItem|get_now_context|useNowContext|onNowUpdate` → 0(timeline 모듈의 get_timeline 제외).

## 5. 범위 밖 (후속 단계)
- `timeline::get_timeline_for_date`/range + `TimelineItem` (3단계 — Header/CommandPalette/WeekView 마이그레이션 후).
- tasks/cards/routines/reports 코어 + 4뷰 (2·4·5단계).
- `V5__drop_legacy.sql` (5단계).
