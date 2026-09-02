# RustyClawd Documentation

**Welcome to RustyClawd documentation!**

This directory contains comprehensive documentation for the RustyClawd project - a Claude Code implementation in Rust with TRUE 100% parity.

---

## Quick Links

### Architecture
- [Main Architecture](./ARCHITECTURE.md) - System overview and design
- [E2E Testing Architecture](./architecture/e2e_testing_architecture.md) - Testing system design
- [Architecture Summary](./architecture/ARCHITECTURE_SUMMARY.md) - Executive summary
- [Design Specifications](./architecture/) - Design docs and specifications

### Testing Documentation
- [E2E Testing Guide](./testing/E2E_TESTING.md) - How to run E2E tests
- [E2E Test Development](./testing/E2E_TEST_DEVELOPMENT.md) - How to write new E2E tests
- [Parity Validation Report](./testing/PARITY_VALIDATION.md) - TRUE 100% parity validation
- [TRUE 100% Parity Validation](./TRUE_100_PARITY_VALIDATION.md) - Final 10% completion evidence
- [Development Resources](./development/) - Test analysis and development documentation

### Implementation
- [Implementation Plan](./implementation_plan.md) - Detailed task breakdown
- [Implementation Reports](./implementation-reports/) - Progress reports

### Archive
- [Point-in-Time Reports](./archive/reports/) - Historical reports and summaries

### Specifications
- [Module Specs](./specs/) - Detailed specifications for key modules
  - [TestSession Spec](./specs/test_session_spec.md)
  - [MockLLM Spec](./specs/mock_llm_spec.md)
  - [tmux Framework Spec](./specs/tmux_framework_spec.md)

### Integration
- [Anthropic Backend Configuration](./reference/ANTHROPIC_CONFIGURATION.md) - Credential, endpoint, model, and provider precedence
- [LiteLLM Gateway](./howto/LITELLM_GATEWAY.md) - Route Anthropic-compatible requests through LiteLLM
- [GitHub Copilot Backend](./copilot-backend.md) - Use GitHub Copilot as the API backend
- [Hook Lifecycle Integration](./HOOK_LIFECYCLE_INTEGRATION.md)
- [HTTP MCP Transport](./HTTP_MCP_TRANSPORT.md)
- [MCP Prompts](./MCP_PROMPTS.md)

---

## Documentation Structure

```
docs/
├── README.md                    # This file
├── ARCHITECTURE.md              # Main architecture
├── HOOK_LIFECYCLE_INTEGRATION.md
├── HTTP_MCP_TRANSPORT.md
├── MCP_PROMPTS.md
│
├── architecture/                # Architecture documents & design specs
│   ├── ARCHITECTURE_SUMMARY.md
│   ├── e2e_testing_architecture.md
│   ├── DESIGN_SPECIFICATION.md
│   └── REQUIREMENTS_CLARIFICATION_MCP_PROMPTS.md
│
├── testing/                     # Testing documentation
│   ├── E2E_TESTING.md          # User guide for running tests
│   ├── E2E_TEST_DEVELOPMENT.md # Developer guide for writing tests
│   └── PARITY_VALIDATION.md    # Parity validation report
│
├── development/                 # Development resources
│   └── ...                     # Test analysis and dev documentation
│
├── specs/                       # Module specifications
│   ├── test_session_spec.md
│   ├── mock_llm_spec.md
│   └── tmux_framework_spec.md
│
├── implementation-reports/      # Implementation progress reports
│   └── ...
│
├── archive/                     # Historical records
│   └── reports/                # Point-in-time reports
│
└── implementation_plan.md       # Detailed implementation plan
```

---

## For Users

**Getting Started:**
1. Read [ARCHITECTURE.md](./ARCHITECTURE.md) for system overview
2. See [E2E Testing Guide](./testing/E2E_TESTING.md) to run tests
3. Check [TRUE 100% Parity Validation](./TRUE_100_PARITY_VALIDATION.md) for parity achievement evidence

**Running Tests:**
```bash
# Run all E2E tests
cargo test --test e2e

# Run specific phases
cargo test --test e2e_programmatic  # Phase 1
cargo test --test e2e_tmux          # Phase 2
cargo test --test e2e_scenarios     # Phase 3
```

---

## For Developers

**Contributing:**
1. Read [E2E Test Development](./testing/E2E_TEST_DEVELOPMENT.md) to write tests
2. Check [Module Specs](./specs/) for implementation details
3. Follow [Implementation Plan](./implementation_plan.md) for structure

**Key Concepts:**
- **Bricks & Studs:** Modular architecture with clear contracts
- **Zero-BS Implementation:** Every function works or doesn't exist
- **TRUE 100% Parity:** Identical behavior to Claude Code

---

## For Architects

**Understanding the System:**
1. [Architecture Summary](./architecture/ARCHITECTURE_SUMMARY.md) - Executive overview
2. [E2E Testing Architecture](./architecture/e2e_testing_architecture.md) - Testing design
3. [Module Specs](./specs/) - Component specifications

**Design Principles:**
- Ruthless simplicity
- Modular design (bricks & studs)
- Quality over speed
- Regeneratable components

---

## Documentation Standards

All documentation in this directory follows the **Eight Rules of Good Documentation**:

1. **Location:** All docs in `docs/` directory ✅
2. **Linking:** Every doc linked from at least one other doc ✅
3. **Simplicity:** Plain language, minimal words ✅
4. **Real Examples:** Runnable code, not placeholders ✅
5. **Diataxis:** One doc type per file ✅
6. **Scanability:** Descriptive headings, TOC for long docs ✅
7. **Local Links:** Relative paths with context ✅
8. **Currency:** Delete outdated docs, include metadata ✅

**Documentation Types:**
- **Tutorials:** Learning-oriented (step-by-step guides)
- **How-To Guides:** Task-oriented (E2E_TESTING.md)
- **Reference:** Information-oriented (specs/)
- **Explanation:** Understanding-oriented (ARCHITECTURE.md)

---

## Status

**Project Status:** TRUE 100% Parity Achieved ✅
**Parity Level:** TRUE 100% (validated through 778+ tests)
**Test Coverage:** 100% passing (778+ tests, 0 failures)
**Last Updated:** 2025-12-07

---

## Questions?

**Architecture Questions:** See [ARCHITECTURE.md](./ARCHITECTURE.md)
**Testing Questions:** See [testing/E2E_TESTING.md](./testing/E2E_TESTING.md)
**Development Questions:** See [testing/E2E_TEST_DEVELOPMENT.md](./testing/E2E_TEST_DEVELOPMENT.md)
**Philosophy Questions:** See `.claude/context/PHILOSOPHY.md`

---

**Welcome aboard, matey!** 🦜⚓
