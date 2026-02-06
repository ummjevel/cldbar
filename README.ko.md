# cldbar

AI 코딩 도구 사용량 모니터. Windows 시스템 트레이에서 Claude, Gemini, z.ai의 토큰 사용량, 활성 세션, 일별 추이를 실시간으로 확인할 수 있습니다.

[English](README.md)

![Tauri v2](https://img.shields.io/badge/Tauri-v2-blue) ![React 19](https://img.shields.io/badge/React-19-61dafb) ![Rust](https://img.shields.io/badge/Rust-2021-orange)

## 주요 기능

- **멀티 프로바이더** — Claude Code, Gemini CLI, z.ai
- **이중 소스 타입** — 로컬 계정 사용량 또는 Claude API 사용량(Admin API 키) 모니터링
- **시스템 트레이** — Windows 트레이에 상주하며 좌클릭으로 팝업 토글, 우클릭으로 종료
- **실시간 통계** — 입출력 토큰, 활성 세션, 메시지 수 (5초마다 자동 새로고침)
- **7일 추이 차트** — 프로필별 일일 사용량 스파크라인
- **API 비용 추적** — Claude API 프로필의 실제 과금 데이터
- **라이트 / 다크 / 시스템 테마** — 블러 효과의 글래스모피즘 UI
- **멀티 프로필** — 여러 프로바이더 프로필 추가, 삭제, 전환

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
  components/tray/          # UI 컴포넌트 (TrayPopup, StatCards, ...)
  hooks/                    # 데이터 페칭 훅
  lib/                      # 타입, 색상, 포맷, 테마
  styles/                   # 테마 변수가 포함된 글로벌 CSS

src-tauri/src/              # Rust 백엔드
  providers/                # 프로바이더 구현체
    claude.rs               # Claude Code (로컬 ~/.claude)
    claude_api.rs           # Claude Admin API
    gemini.rs               # Gemini CLI (로컬 ~/.gemini)
    zai.rs                  # z.ai (로컬 %APPDATA%/zai)
    mod.rs                  # Provider 트레이트
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
- `~/.gemini/` → Gemini
- `%APPDATA%/zai/` → z.ai

설정 패널에서 추가 프로필(Claude API 포함)을 등록할 수 있습니다.

## 라이선스

MIT
