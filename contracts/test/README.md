# JS dependencies for FFI tests

FFI 통합 테스트에서 `@aztec/bb.js`, `@noir-lang/noir_js`, `ethers`를 사용하므로 이 디렉토리에서 별도 npm 의존성을 관리합니다.

## 설치
```bash
cd contracts/test
npm install
```

## 사용 예시
- FFI 테스트 실행: `cd .. && FOUNDRY_FFI=1 forge test --ffi`
- 단독 proof 생성: `npm run proof -- <args>` (스크립트: `scripts/generate_proof.js`)

Note: Node 18+ 기준, ESM(`type: "module"`)을 사용합니다. CI에서도 `npm ci` 후 `forge test --ffi`를 수행하세요.
