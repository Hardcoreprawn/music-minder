# Agent Workflow & Instructions

This document outlines how AI agents (and human developers) should contribute to the Music Minder project.

## Philosophy
- **Test-Driven**: Write tests for logic *before* or *alongside* implementation.
- **Modular**: Keep modules loosely coupled.
- **Stateless**: Agents should be able to pick up a task with just the context of the repo.

## Workflow for New Features

### 1. Pickup
- Read `docs/ROADMAP.md` to find the next pending User Story.
- Mark the story as "In Progress" (if possible, or note it in the response).

### 2. Context Gathering
- Read `docs/ARCHITECTURE.md` to understand where the new feature fits.
- Check existing code in `src/` to match style and patterns.

### 3. Implementation
- **Create a new module** if necessary.
- **Define types** in `src/model/` if data structures change.
- **Implement logic**.
- **Add Unit Tests**: Every core function must have a `#[test]`.

### 4. Quality Assurance (QA)
- Run `cargo test` to ensure no regressions.
- Run `cargo clippy` to ensure idiomatic Rust.
- If the feature is UI-related, describe the expected visual outcome.

### 5. Handoff
- Update `docs/ROADMAP.md` marking the item as "Completed".
- Update `docs/CHANGELOG.md` (if it exists) or add a summary of changes.

## Agent Personas

### Developer Agent
- **Role**: Implement features.
- **Focus**: Correctness, Error Handling, Performance.
- **Output**: Code changes, Unit tests.

### QA Agent
- **Role**: Review and Test.
- **Focus**: Edge cases, Integration tests, User Experience.
- **Output**: Test reports, Bug fixes, Refactoring suggestions.

## Common Commands
- Build: `cargo build`
- Test: `cargo test`
- Lint: `cargo clippy`
- Run: `cargo run`
