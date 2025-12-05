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
# 인덱서까지 돌리려면 (WS 엔드포인트/컨트랙트 주소 필요)
# RPC_WS=ws://localhost:8545 CONTRACT_ADDRESS=0x... INDEXER_FROM_BLOCK=0 cargo run
```

또는 Docker:
```bash
cd infra
docker compose up --build
```

### Demo 데이터 시드
로컬 데모 화면을 바로 보고 싶다면 backend 루트에서 아래 스크립트를 실행하세요.  
`psql` 클라이언트가 필요하며 `DATABASE_URL`을 지정하지 않으면 `postgres://veilcast:veilcast@localhost:5432/veilcast`가 사용됩니다.  
로컬에 `psql`이 없어도 Docker가 설치되어 있다면 자동으로 `docker compose exec db psql`을 사용합니다.
```bash
cd backend
chmod +x scripts/seed_demo_data.sh # 최초 1회
DATABASE_URL=postgres://veilcast:veilcast@localhost:5432/veilcast scripts/seed_demo_data.sh
```
스크립트는 2025년 9월 이후 이미 결론이 난 글로벌 이슈(Nvidia 시총 1위, Apple $4T, Bitcoin ATH/하락, Solana ETF, BoA 크립토 개방, 2025 World Series 등)를 다루는 resolved poll 7개와 데모 커밋/투표, 리더보드 데이터를 한 번에 만들어 줍니다.  
시드가 끝나면 일반 `cargo run` 혹은 docker dev 프로파일을 그대로 실행하면 됩니다.

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
- 인덱서: `ethers-rs` WS 로그 구독 → `PollIndexSink` trait을 통해 DB에 반영 (PollCreated, VoteRevealed, PollResolved).
