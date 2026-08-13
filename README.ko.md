# cldbar

AI 코딩 도구 사용량 모니터. Windows 시스템 트레이에서 플랜 한도가 얼마나 남았고 언제 리셋되는지를, Claude·Codex·Gemini의 토큰 사용량·활성 세션·일별 추이와 함께 확인할 수 있습니다.

[English](README.md)

![Tauri v2](https://img.shields.io/badge/Tauri-v2-blue) ![React 19](https://img.shields.io/badge/React-19-61dafb) ![Rust](https://img.shields.io/badge/Rust-2021-orange)

## 주요 기능

- **멀티 프로바이더** — Claude Code, Codex CLI, Gemini CLI
- **한도를 최상단에** — 세션·주간 창이 팝업 맨 위에 오고, 남은 양과 리셋까지 남은 시간을 같은 무게로 보여줍니다. 어느 쪽이든 마우스를 올리면 반대 관점(사용한 양)으로 뒤집어 보여줍니다.
- **사용량 알림** — 임계값을 넘거나 리셋이 임박하면 토스트로 알립니다. 임계값·리마인더·대상 계정·주기를 설정할 수 있습니다.
- **시스템 트레이** — Windows 트레이에 상주하며 좌클릭으로 팝업 토글, 우클릭으로 종료
- **통계** — 입출력 토큰, 활성 세션, 메시지 수
- **7일 추이 차트** — 프로필별 일일 사용량 스파크라인
- **라이트 / 다크 / 시스템 테마** — 블러 효과의 글래스모피즘 UI
- **멀티 프로필** — 여러 프로바이더 프로필 추가, 삭제, 전환

## 한도와 알림

### 데이터 출처

| 프로바이더 | 출처 |
|-----------|------|
| Claude (계정) | Anthropic OAuth usage 엔드포인트. Claude Code가 `~/.claude/.credentials.json`에 저장한 토큰을 사용합니다. |
| Codex (계정) | ChatGPT usage 엔드포인트. Codex가 `~/.codex/auth.json`에 저장한 토큰을 사용하며, 로그인을 쓸 수 없으면 rollout 로그에 기록된 마지막 스냅샷으로 대체합니다. |
| z.ai (API) | z.ai quota 엔드포인트 |

토큰 수·세션·일별 추이는 로컬 로그(`~/.claude/projects`, `~/.codex/sessions`, `~/.gemini`)에서 읽으며 이 컴퓨터 밖으로 나가지 않습니다.

### 언제 읽는가

- **팝업을 열 때**와 새로고침 버튼을 누를 때. 새로고침은 명시적 요청이므로 캐시를 우회합니다.
- **알림 엔진의 자체 주기**로 백그라운드에서. 팝업이 닫혀 있어도 알림이 동작합니다. 기본값은 15분입니다.

조회 결과는 팝업과 알림 엔진이 공유하며 2분간 캐시되므로, 팝업을 자주 여닫아도 추가 요청이 발생하지 않습니다. 응답을 거부하는 프로바이더는 간격을 점점 벌리며 재시도합니다(최대 30분).

> 위 usage 엔드포인트들은 비공개·비문서화 API입니다. 자주 호출하면 rate limit이 걸리고, 한 번 걸리면 **호출을 멈출 때까지 계속 거부**합니다. 이때 화면에는 **No limit data**로 표시됩니다. 알림 주기는 몇 분 단위로 유지하세요 — 선택지에 1분 미만이 없는 이유입니다. 또한 이 엔드포인트들은 예고 없이 바뀔 수 있습니다.

한도를 스트리밍이 아니라 주기적으로 샘플링하므로, 알림은 **최대 한 주기만큼 늦게** 도착할 수 있습니다. 리셋 리마인더는 정확한 분이 아니라 **구간으로 판정**합니다 — 설정한 시점으로부터 한 주기 이내에 들어오면 해당 리마인더로 보고 발송하므로, "1시간 전"은 1시간 근처에 도착합니다. 구간을 놓친 리마인더는 뒤늦게 보내지 않고 그냥 버립니다.

## 기술 스택

| 레이어 | 기술 |
|--------|------|
| 데스크톱 런타임 | Tauri v2 |
| 백엔드 | Rust (reqwest, rusqlite, chrono, serde) |
| 프론트엔드 | React 19 + TypeScript |
| 스타일링 | Tailwind CSS v4 + Framer Motion |
| 차트 | Recharts |

## 프로젝트 구조

```
src/                        # React 프론트엔드
  components/tray/          # 팝업 UI (TrayPopup, LimitWindows, StatCards, ...)
  components/alert/         # 토스트 오버레이 창 (AlertOverlay, AlertToast)
  hooks/                    # 데이터 페칭 훅
  lib/                      # 타입, 색상, 포맷, 한도 계산, 테마
  styles/                   # 테마 변수가 포함된 글로벌 CSS

src-tauri/src/              # Rust 백엔드
  providers/                # 프로바이더 구현체
    claude.rs               # Claude Code (로컬 ~/.claude)
    claude_api.rs           # Claude Admin API
    codex.rs                # Codex CLI (로컬 ~/.codex)
    gemini.rs               # Gemini CLI (로컬 ~/.gemini)
    zai.rs                  # z.ai (로컬 %APPDATA%/zai)
    mod.rs                  # Provider 트레이트, 공용 HTTP 클라이언트
  alerts.rs                 # 한도 폴링, 알림 규칙, 토스트 창
  commands.rs               # Tauri IPC 커맨드
  profile.rs                # 설정 파일 관리
  lib.rs                    # 앱 설정 및 트레이 로직
```

## 시작하기

### 사전 요구사항

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://rustup.rs/)
- [Tauri v2 사전 요구사항](https://v2.tauri.app/start/prerequisites/) (Windows의 WebView2)

### 개발 모드

```bash
npm install
npm run tauri dev
```

### 릴리즈 빌드

```bash
npm run tauri build
```

빌드 결과물:

| 파일 | 경로 |
|------|------|
| EXE (단독 실행) | `src-tauri/target/release/cldbar.exe` |
| NSIS 설치 프로그램 | `src-tauri/target/release/bundle/nsis/cldbar_*-setup.exe` |
| MSI 설치 프로그램 | `src-tauri/target/release/bundle/msi/cldbar_*.msi` |

## 설정

설정 파일은 `%APPDATA%/cldbar/config.json`에 저장됩니다. 첫 실행 시 설치된 프로바이더를 자동으로 감지합니다:

- `~/.claude/` → Claude
- `~/.codex/` → Codex
- `~/.gemini/` → Gemini
- `%APPDATA%/zai/` → z.ai

설정 파일이 만들어진 뒤에 추가된 프로바이더는 **한 번만** 제안되고 그 사실이 기록되므로, 삭제한 프로필이 다음 실행 때 다시 살아나지 않습니다.

설정 패널에서 추가 프로필(Claude API 포함)을 등록할 수 있습니다. 테마, 한도를 "남은 양"으로 볼지 "사용한 양"으로 볼지, 알림 규칙도 설정 패널에서 조정합니다.

## 라이선스

MIT
