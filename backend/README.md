# VeilCast Backend (Rust)

경량 axum 기반 API/인덱싱 스켈레톤입니다.

## 주요 책임
- Poll 메타데이터 저장 (Postgres)
- 커밋/널리파이어 기록
- ZK 백엔드 추상화(`ZkBackend`): 현재는 `NoopZkBackend`로 SHA256 기반 모의 증명
- HTTP 라우트: `/health`, `/polls`, `/polls/:id`, `/polls/:id/commit`, `/polls/:id/prove`, `/polls/:id/reveal`

## 실행
```bash
cd backend
DATABASE_URL=postgres://veilcast:veilcast@localhost:5432/veilcast cargo run
```

또는 Docker:
```bash
cd infra
docker compose up --build
```

## 테스트
```bash
cd backend
cargo test
```

## 아키텍처 메모
- `AppState<Store, Backend>`: `PollStore`(Postgres/InMemory)와 `ZkBackend`(Noop) 제너릭 상태.
- `PollStore` trait으로 DB 추상화, `PgStore`는 기본 schema 자동 생성.
- `ZkBackend` trait으로 프루프 생성/검증을 모듈화. 실제 bb.js 연동 시 이 부분 교체.
- Phase는 현재 시각으로 계산하여 commit/reveal 윈도우 검증.
