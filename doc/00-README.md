# OxiLine 설계 문서 세트

이 폴더는 macOS용 루틴/하루 관리 앱 **OxiLine**의 그린필드(greenfield) 구현을 위한 전체 설계 문서입니다.
아래 순서대로 읽으면 제품 철학 → 리서치 → 데이터 모델 → 아키텍처 → CLI → 디자인 시스템 → 화면 설계 → 로드맵까지
하나의 논리로 이어지도록 구성했습니다. 이 문서 세트 전체를 원샷 프롬프트의 컨텍스트로 사용하는 것을 전제로,
**애매한 부분을 남기지 않고 구체적인 스키마·명령어·토큰 값까지 명시**했습니다.

## 문서 목록

| 파일 | 내용 |
|---|---|
| `01-product-vision.md` | 제품 철학, 포지셔닝, Oxi 생태계 내 위치, 용어 정의, Non-goals |
| `02-ux-research.md` | Structured / Sunsama / Amie / 한국 "갓생" 루틴 앱 리서치 및 디자인 원칙 도출 |
| `03-data-model.md` | 핵심 엔티티, ERD, SQLite 스키마 DDL, 가상 occurrence/materialize 전략 |
| `04-architecture.md` | Rust workspace 구조, Tauri v2 앱 아키텍처, 트레이/전역 단축키/플로팅 HUD, IPC, 동기화 |
| `05-cli-spec.md` | CLI 명령어 전체 스펙, JSON 출력 스키마, 에이전트 연동 시나리오, MCP 확장안 |
| `06-design-system.md` | OKLCH 컬러 토큰, 타이포그래피, 모션, 시그니처 비주얼(Oxide Bar) |
| `07-ui-screens-and-flows.md` | 화면별 상세 설계, 주요 플로우, 컴포넌트 명세 |
| `08-roadmap.md` | 구현 단계(Phase 0~3), 각 단계별 완료 기준 |

## 이 문서를 원샷 프롬프트로 쓸 때

1. 이 폴더 전체를 그린필드 리포지토리 루트에 `docs/` 등으로 두고, "이 문서들을 스펙으로 삼아 OxiLine을 구현해줘"라고
   요청하면 됩니다.
2. 구현 우선순위는 `08-roadmap.md`의 Phase 0 → Phase 1(MVP) 순서를 따르세요. Phase 2/3은 MVP 이후입니다.
3. 프로젝트명, 바이너리명, 번들 ID는 아래 확정값을 그대로 사용합니다 (Oxi 생태계 네이밍 컨벤션과 통일).

## 확정 네이밍

- 제품명: **OxiLine**
- macOS 앱 번들: `OxiLine.app`, bundle identifier `com.oxi.oxiline`
- GUI 실행 파일 내부명: `oxiline`
- CLI 바이너리명: `oxiline` (PATH에 별도 설치, GUI 앱 번들 내부 실행파일과 경로가 달라 충돌 없음)
- Cargo workspace 루트 이름: `oxiline`
- 데이터 저장 위치: `~/Library/Application Support/OxiLine/oxiline.db` (자세한 내용은 `04-architecture.md`)

## 핵심 한 줄 요약

> OxiLine은 "일정"이 아니라 "하루가 흘러가는 방식"을 관리하는 앱이다. 시간은 재생 헤드처럼 끊임없이 흐르고,
> 루틴은 그 흐름 위에 놓인 레인이며, 전역 단축키는 지금 이 순간으로 즉시 돌아오는 문이다.
