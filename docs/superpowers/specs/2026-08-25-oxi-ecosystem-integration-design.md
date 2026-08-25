# OxiLine × Oxi 생태계 통합 설계

> **Status:** Approved · **Date:** 2026-08-25
> **Scope:** OxiLine을 `.oxi/` 기반 Oxi 생태계(oxibrain, `.oxi/vault`, oxios, oxi 터미널
> 에이전트)에 어떻게 연결할지에 대한 결정. 구현 스코프는 에이전트 연동 계층 하나
> (`oxiline-cli` skill)로 좁힌다.

## 배경

사용자가 Oxi 생태계를 `.oxi/`(설정, `brain/`의 oxibrain 데몬+SQLite, `vault/`의 마크다운
노트) 아래로 통합하는 중이며, OxiLine도 이 흐름에 태울지 검토를 요청했다. 후보로 세 가지가
논의됐다: (1) 일정 데이터를 `.oxi/vault/`에 마크다운으로 저장(Obsidian Tasks/Logseq 스타일),
(2) SQLite를 유지하되 oxibrain에 행동 데이터를 ingest, (3) 둘 다 하지 않고 별도 유지.

## 조사한 사실

1. **`.oxi/vault/`는 RFC-050(Accepted, 2026-08-22)으로 포맷이 확정된 자유형 노트 저장소다.**
   `oxi-frontmatter` 그래머(`---` YAML, 깊이 2까지 중첩, id/created/updated/favorite/deleted
   + 열린 app 테이블)는 관계형 데이터를 표현하지 못한다. OxiLine의 실제 스키마(`activities`,
   `records`, `plans`/`plan_options`)는 분 단위 정수, 요일 반복 마스크, FK, 유니크 제약을
   갖는 진짜 관계형 모델이라 이 그래머에 태우면 핵심 로직(예: OR 선택지 구체화)을 잃는다.
2. **oxios ↔ OxiLine은 이미 "co-client SQLite" 패턴으로 실제 통합돼 있다**
   (`../oxios/docs/designs/2026-08-05-app-module-integration-design.md`, 구현·검증 완료).
   명시적 제약 C1: *"oxiline must NOT become the calendar/scheduler backend. oxios is a
   co-client of each app's own canonical store, never the owner."* `oxiline-core`
   (crates.io 0.3.1)를 oxios가 직접 의존해 같은 SQLite 파일에 붙는다.
3. **oxios RFC-047**은 생태계 재편 방향을 "oximemo absorbing authoring, oxiline absorbing
   time features"로 명시한다 — OxiLine이 vault에 종속되는 방향이 아니라 시간 도메인을
   흡수하는 쪽으로 이미 정책화돼 있다.
4. **oxibrain은 관계형 스케줄 데이터의 백엔드가 아니다.** entity/statement/belief 기반
   episodic memory이며, 소비 경로는 `Brain::ingest`(텍스트 투입) 또는 vault 디렉터리
   `sync_run`(watch) 둘뿐이다.
5. **oxiline은 이미 CLI-first 정체성을 갖고 있다** (`doc/01-product-vision.md` §1.3, §1.5).
   Non-goal로 "AI 자동 일정 생성"을 명시하며 "CLI를 통해 사용자의 개인 에이전트가 이 역할을
   대신하게 하는 것이 철학과 더 맞다"고 못 박는다. 로드맵 Phase 3에 `oxiline mcp serve`가
   "평가 후 채택" 항목으로 예비돼 있었다.
6. **실제 구현은 원 설계 문서(`03-data-model.md`의 routine/task 모델)에서 레코딩 중심
   모델로 피벗되어 있다** — 실제 CLI(`crates/oxiline-cli/src/cli.rs`, 2026-08-25 기준)는
   `now / category / activity / settings / hud / report / record / plan / upgrade /
   doctor`이고, `doc/05-cli-spec.md`(routine/task 트리)는 이 피벗 이후 갱신되지 않아 stale
   하다. 본 설계와 이후 산출물은 문서가 아니라 실제 코드를 그라운딩으로 삼는다.
7. **`.agent/skills/atelier-cli/SKILL.md`가 이미 이 레포에 선례로 존재한다** — CLI를
   감싸는 SKILL.md 한 장으로 에이전트에게 도구를 쥐어주는 패턴이 이 워크플로우 환경
   (OMP)에서 이미 검증되어 있고, 시스템 프롬프트의 전역 skill 목록에도 동일 내용이 노출된다
   (managed skill로 등록되어 있음을 시사).

## 결정

| 축 | 결정 |
|---|---|
| 저장소 | SQLite(`oxiline-core`) 유지. `.oxi/vault` 마크다운 이관 안 함 |
| oxibrain 데이터 브리지 (회상 방향: OxiLine 기록 → 에이전트 기억) | **지금 만들지 않는다.** 구체적 사용 사례가 생기면 oxios 쪽 `TimelineApi` + `BrainClient`에 opt-in 훅으로 추가 — OxiLine 코드 변경 0으로 가능하므로 지금 만들지 않는 결정에 매몰비용이 없다 |
| 에이전트 행동 방향 (에이전트 → OxiLine 조작) | `oxiline mcp serve` 대신 **CLI + SKILL.md**. 상주 프로세스 없이 이미 완비된 `--json`/exit-code 계약을 그대로 재사용 |
| MCP | Phase 3 "평가 후 채택" 항목 평가 완료 → 보류. Claude Desktop처럼 셸 실행이 불가능한 MCP 전용 호스트가 실제로 필요해지는 시점에 재검토(구현 시 core crate 변경 없이 `oxiline-cli`에 `rmcp` 추가만 하면 되므로 나중에 붙여도 비용 동일) |

### 비대칭성에 대한 판단

OxiLine → oxibrain 데이터 흐름은 oxibrain(생태계 기억)만 이득을 보고 OxiLine 자신은
아무것도 얻지 못하는 비대칭 관계다. 이 비대칭 자체가 "아직 때가 아니다"라는 신호로 읽었다 —
oxios→OxiLine, OxiLine→에이전트(CLI/skill) 방향은 양쪽 다 이유가 명확한데 반해, 이 방향은
한쪽에만 이유가 있다. 반대로 CLI+skill 방향은 OxiLine 자신의 정체성(§1.5 "CLI + 에이전트
연동")과 정확히 부합하는 순수 이득이라 우선순위를 뒀다.

## 구현 스코프

### 1. `.agent/skills/oxiline-cli/SKILL.md`

`atelier-cli` skill과 동일 포맷(frontmatter name/description + 트리거 문구, 명령 요약,
`--json` 계약, 종료 코드, 워크플로우 예시, 하지 말아야 할 것). 실제 실행한 CLI 출력을
그라운딩으로 삼아 작성한다(`doc/05-cli-spec.md`의 stale한 routine/task 모델이 아님).

### 2. 전역 managed skill 등록

`manage_skill`(action: create)로 동일 내용을 전역 등록 — 프로젝트 디렉터리와 무관하게 어떤
OMP 세션에서도 discoverable하도록. `atelier-cli`가 이 세션 시스템 프롬프트에 전역으로
뜨는 것과 동일한 메커니즘.

### 3. 문서 갱신 (이 결정을 반영)

- `doc/01-product-vision.md` §1.3에 생태계 통합 경계 결정을 추가.
- `doc/08-roadmap.md` Phase 3의 `oxiline mcp serve` 항목에 평가 결과 주석.
- `doc/05-cli-spec.md`에 staleness 안내 배너 추가(실제 명령 트리는 `cli.rs`와
  `.agent/skills/oxiline-cli/SKILL.md`가 최신 소스임을 명시). 전체 재작성은 본 설계의
  스코프 밖(별도 작업).
- `CHANGELOG.md`에 skill 추가 항목 기록.

## 비범위 (Non-goals)

- `.oxi/vault/`로의 데이터 이관.
- oxibrain ingest 브리지 구현(oxios 쪽 작업이며, 이 설계의 스코프 밖).
- `oxiline mcp serve` 구현.
- `doc/05-cli-spec.md` 전체 재작성(실제 CLI에 맞춘 재작성은 별도 프로젝트).

## 검증 계획

- SKILL.md에 적은 모든 명령 예시를 실제 `cargo run -p oxiline-cli -- ...`로 실행해 출력이
  일치하는지 확인(스모크 테스트, 신규 자동 테스트 불필요 — 문서 산출물이므로).
