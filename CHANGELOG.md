# Changelog

All notable changes to ZeroClaw (qmzz fork) will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Planned
- Stabilization and regression testing for P0/P1 enhancements
- Documentation improvements for Chinese users
-专项功能测试覆盖 (doctor/status/layered-config/failure-recovery/policy-engine)

---

## [0.7.1-patched] - 2026-04-13

### ✨ Added — P0/P1 Enhancements (Claw Code-inspired)

#### Diagnostic & Health Layer
- **Enhanced `/doctor` command**: comprehensive system diagnostics with memory/tools/resources/provider checks
- **JSON output support**: `doctor --format json` for programmatic access
- **Fix mode**: `doctor --fix` for automated recovery attempts
- **Enhanced `/status` command**: human/json/brief output formats, watch mode, file output
- **Health monitoring subsystem**: `src/health/mod.rs` with component health tracking
- **Failure registry**: `src/health/failure.rs` with structured failure classification
- **Recovery engine**: `src/health/recovery.rs` with automated recovery paths

#### Configuration Layer
- **Layered configuration**: `src/config/layered.rs` supporting workspace/project/local override hierarchy
- **Fixed default value pollution**: merged based on raw TOML values instead of default-tainted structs
- **Preserved runtime paths**: `workspace_dir` and `config_path` retained through layer merging

#### Security & Policy Layer
- **Policy engine**: `src/security/policy_engine.rs` with `EvaluationContext` and `PolicyDecision`
- **Security policy integration**: `SecurityPolicy::validate_command_execution_with_context`接入
- **Tool call chain injection**: shell/cron/skill_tool chains now pass through policy evaluation

#### Context Management
- **Deterministic compactor**: `src/agent/deterministic_compactor.rs` for predictable compression
- **Integration points**:接入 `agent::loop_.rs::build_context` and `channels::mod.rs::build_memory_context`
- **Compression budget control**: `CompressionBudget` struct for token-aware truncation

#### Failure Classification
- **FailureKind enum**: Provider, Compact, Session, Channel, Memory, Config, Tool, Resource, Unknown
- **FailureSeverity enum**: Low, Medium, High, Critical
- **FailureRecord structure**: with fingerprinting, occurrence tracking, resolution status
- **Failure filter & query**: `FailureFilter` for targeted failure retrieval

### 🔧 Fixed

#### Compilation & Tests
- Fixed `status/mod.rs`: added `crate::memory` import, fixed `channels().into_iter()`, fixed `libc::statvfs` private fields
- Fixed `doctor/mod.rs`: removed non-existent field accesses (`memory_dir`/`memory_backend`/`custom_providers`/`enabled_tools`/`disabled_tools`), simplified `FixAction`, fixed `statvfs`
- Fixed `config/layered.rs`: changed to raw TOML value merge to avoid default value pollution
- Fixed `security/mod.rs`: resolved `PolicyDecision` duplicate export conflict
- Fixed `main.rs`: layered config loading logic, cleaned up warnings

#### Test Coverage
- All `cargo check` checks pass
- All `cargo test --lib --no-run` tests pass
- All `doctor::tests::*` 21/21 tests pass
- Partial security tests pass

### 🚀 Deployment

#### Docker & CI/CD
- GitHub Actions Docker image build: successful (11 minutes)
- Image pushed to GHCR: `ghcr.io/qmzz/zeroclaw:test`
- Container startup: verified
- Basic conversation: verified
- QQ official bot integration: verified (app_id + app_secret)
- Cost tracking: usage data displayed (prices require manual `[cost.prices]` config)

#### Verified Performance
- Image size: ~60MB
- Runtime memory: ~6MB
- Cold start: <10ms

### 📝 Documentation

#### New Documents
- `report/zeroclaw-fix-status-and-external-test-handoff-2026-04-11.md`: compilation fix and external test handoff
- `report/zeroclaw-acceptance-summary-2026-04-13.md`: P0/P1 acceptance summary
- `report/zeroclaw-phase2-assessment-2026-04-13.md`: Phase 2 necessity assessment (recommendation: defer)
- `report/claw-code-architecture-comparison.md`: Claw Code architecture comparison
- `report/claw-code-source-deep-dive.md`: Claw Code source code deep dive
- `report/claw-code-to-copaw-roadmap.md`: transformation roadmap

#### Updated Documents
- `README.md`: added Chinese quick-start section with P0/P1 highlights
- `memory/2026-04-08.md` to `memory/2026-04-13.md`: daily progress logs

### 🎯 Assessment Conclusion

**Phase 2 transformation recommendation**: **DEFER**

Rationale:
- 80%+ of Claw Code's core value has been absorbed in P0/P1
- Current version satisfies real user requirements
- Phase 2 marginal benefit < cost (complexity increase, maintenance burden)
- Risk of breaking zeroclaw's lightweight characteristics
- Future needs can be met through configuration extension + incremental additions

**Next steps**:
- Complete specialized functional testing (doctor/status/layered-config/failure-recovery/policy-engine)
- Supplement regression test coverage
-完善 documentation (README/CHANGELOG/CONFIG)
- Tag current version as `v0.7.1-patched`

---

## [0.6.8] - 2026-04-08

### Baseline
- ZeroClaw upstream v0.6.8 forked as baseline for P0/P1 transformation
- Total codebase: ~290K lines of Rust
- Original features: multi-channel support, web dashboard, cron, skills, tools, memory, providers

---

## Notes

### Version Numbering
- `0.6.x`: upstream ZeroClaw versions
- `0.7.x-patched`: qmzz fork with P0/P1 Claw Code-inspired enhancements
- Future versions will follow semantic versioning based on breaking/feature/patch changes

### Transformation Principles
1. **Absorb Claw Code philosophy, not copy implementation**
2. **Maintain zeroclaw's lightweight characteristics** (image ~60MB, memory ~6MB)
3. **Incremental extension over refactoring**
4. **Configuration-first for flexibility**
5. **Test coverage grows with features**

### Testing Strategy
- Unit tests for core modules (doctor, health, config, security)
- Integration tests for tool call chains
- Container-based deployment verification
- Specialized functional testing for new commands

### Future Considerations (Phase 2 - Deferred)
- Worker lifecycle management (state machine, failure classification automation)
- Verifiable closed loop (health checks, regression lists, automated harness)
- Multi-agent role division (Coordinator + Monitor + Maintainer + Archiver + Healer)
- Heartbeat governance upgrade (5 task categories, status/history commands)
- Policy rule engine (PolicyRule/Condition/Action data structures)

**Trigger conditions for Phase 2**:
1. Performance bottleneck: current architecture cannot support real user load
2. Feature gap: clear requirements cannot be met through incremental extension
3. Stability issue: frequent failures that cannot be resolved through fixes
4. Explicit user request: strategic direction adjustment

---

*For more detailed technical notes, see daily logs in `memory/YYYY-MM-DD.md` and reports in `report/*.md`.*
