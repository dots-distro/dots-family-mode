# BDD Workflow Guide

This document describes the Behavior-Driven Development (BDD) workflow implemented in engram for the Time Window Enforcement feature.

## Workflow Overview

**Workflow ID:** `b90ee9ad-2d1e-4f17-ba78-06d980fc75b8`  
**Instance ID:** `e5f2e4dc-beb3-411a-a443-6257f5b452a7`  
**Associated Task:** `e642def8-7b2d-4828-bc42-c333d8d589d4` (Implement Time Window Enforcement with BDD)

## Workflow States

The workflow defines 7 states following the BDD Red-Green-Refactor cycle:

### 1. specification (Start State)
**State ID:** `c1def180-abc2-4155-94de-0e639457206c`  
**Type:** Start  
**Mapped Task:** `75aef678` - Write Gherkin feature specs

**Purpose:** Writing Gherkin feature specifications

**Commit Policy (Agent-Enforced):**
- ✅ Engram entity changes (tasks, context, reasoning, relationships)
- ❌ Code commits blocked
- ❌ Config/build file changes blocked
- ❌ Test code blocked

**Allowed Activities:**
- Creating Gherkin `.feature` files
- Documenting requirements and scenarios
- Creating engram context entities for requirements
- Creating engram reasoning entities for design decisions

**Transition Out:** `specs_approved` (to framework_setup)

---

### 2. framework_setup (In Progress)
**State ID:** `e87814eb-3988-46c2-9357-971ca3be0953`  
**Type:** InProgress  
**Mapped Task:** `c3c4d6c8` - Set up Cucumber/BDD test framework

**Purpose:** Setting up BDD testing infrastructure

**Commit Policy (Agent-Enforced):**
- ✅ Engram entity changes
- ✅ Config file changes (Cargo.toml, package.json, etc.)
- ✅ Build file changes (flake.nix, build scripts)
- ❌ Feature code blocked
- ⚠️ Test framework setup code allowed

**Allowed Activities:**
- Adding BDD dependencies to Cargo.toml
- Configuring Cucumber or similar framework
- Setting up test directory structure
- Creating test runner scripts

**Transition Out:** `framework_ready` (to red_phase)

---

### 3. red_phase (In Progress)
**State ID:** `d9fb8215-b212-4201-8213-c44c25d1f853`  
**Type:** InProgress  
**Mapped Task:** `0390c634` - Write step definitions

**Purpose:** RED phase - Writing step definitions, expecting failing tests

**Commit Policy (Agent-Enforced):**
- ✅ Engram entity changes
- ✅ Config/build changes
- ✅ Test code (step definitions)
- ❌ Feature implementation code blocked
- ⚠️ Code must compile
- ⚠️ Tests must fail (no feature implementation yet)

**Allowed Activities:**
- Writing step definition files
- Creating test fixtures and mocks
- Implementing Given/When/Then steps
- Running tests to verify they fail appropriately

**Validation Before Transition:**
- Tests compile successfully
- Tests run and fail as expected
- All scenarios properly mapped to step definitions

**Transition Out:** `tests_failing` (to green_phase)

---

### 4. green_phase (In Progress)
**State ID:** `962a94e6-c674-4476-8491-90b4f14ad88c`  
**Type:** InProgress  
**Mapped Task:** `c2aa22fd` - Implement time window enforcement logic

**Purpose:** GREEN phase - Implementing feature to make tests pass

**Commit Policy (Agent-Enforced):**
- ✅ All changes allowed
- ✅ Feature implementation code
- ✅ Test code
- ⚠️ All tests must pass before transition

**Allowed Activities:**
- Implementing time window enforcement logic
- Adding necessary data structures
- Integrating with existing systems
- Iterating until all tests pass

**Validation Before Transition:**
- All BDD tests passing
- All existing tests still passing
- No compiler warnings (unless explicitly documented)
- Code compiles and runs

**Transition Out:** `tests_passing` (to refactor_phase)

---

### 5. refactor_phase (In Progress)
**State ID:** `a1b12384-7b4e-4537-a0a8-317add9fcd3d`  
**Type:** InProgress  
**Mapped Task:** `ce02255a` - Refactor implementation

**Purpose:** REFACTOR phase - Cleaning up implementation while keeping tests green

**Commit Policy (Agent-Enforced):**
- ✅ All changes allowed
- ⚠️ Tests must remain passing after each commit
- ⚠️ No behavior changes (tests should not need modification)

**Allowed Activities:**
- Code cleanup and simplification
- Extracting functions/modules
- Improving naming and documentation
- Performance optimizations
- Removing duplication

**Validation Before Transition:**
- All tests still passing
- Code quality improved
- No behavior regressions
- Documentation updated

**Transition Out:** `refactoring_complete` (to integration_testing)

---

### 6. integration_testing (Review)
**State ID:** `a580cf37-a7a3-4498-b7e4-2a136538d8fb`  
**Type:** Review  
**Mapped Task:** `920724a3` - Add time window tests to NixOS VM

**Purpose:** Integration testing in full NixOS environment

**Commit Policy (Agent-Enforced):**
- ✅ All changes allowed
- ⚠️ Full build must succeed (`nix build`)
- ⚠️ All tests must pass (`nix build .#checks`)
- ⚠️ VM integration tests must pass

**Allowed Activities:**
- Adding VM-based integration tests
- Testing in full NixOS environment
- Performance validation
- End-to-end scenario testing
- Documentation finalization

**Validation Before Transition:**
- VM tests passing
- Performance acceptable
- Feature working in realistic environment
- Documentation complete

**Transition Out:** `validation_complete` (to completed)

---

### 7. completed (Final State)
**State ID:** `fb6d09b4-8c0f-4664-89b3-0b94c5640aed`  
**Type:** Done (Final)

**Purpose:** Feature fully implemented, tested, and validated

**No restrictions** - Feature is complete

---

## Transitions

The workflow defines 6 main transitions between states:

1. **specs_approved**: specification → framework_setup
2. **framework_ready**: framework_setup → red_phase
3. **tests_failing**: red_phase → green_phase
4. **tests_passing**: green_phase → refactor_phase
5. **refactoring_complete**: refactor_phase → integration_testing
6. **validation_complete**: integration_testing → completed

## Agent Enforcement Model

Since engram workflow instances currently start in an "initial" state and transitions are not automatically executing, **OpenCode agent will enforce the workflow rules manually:**

### How It Works

1. **State Tracking**: The agent tracks which workflow state we're in based on which task is active
2. **Commit Validation**: Before any commit, the agent checks:
   - Current workflow state (based on active task)
   - Commit policy for that state
   - Files being committed
   - Test results (if required)
3. **Transition Execution**: When a task is completed:
   - Agent verifies transition criteria met
   - Agent marks task as done
   - Agent manually executes workflow transition (if needed)
   - Agent moves to next task in sequence
4. **Enforcement**: Agent will refuse to commit code that violates current state's policy

### Current State

**We are currently in:** specification state  
**Active task:** `75aef678` (Write Gherkin feature specs)  
**Allowed:** Only engram entities and Gherkin `.feature` files  
**Blocked:** All code commits

### State-Task Mapping

| Workflow State | Task ID | Task Description |
|----------------|---------|------------------|
| specification | 75aef678 | Write Gherkin feature specs |
| framework_setup | c3c4d6c8 | Set up Cucumber/BDD test framework |
| red_phase | 0390c634 | Write step definitions - expect RED |
| green_phase | c2aa22fd | Implement logic - make tests GREEN |
| refactor_phase | ce02255a | Refactor while keeping tests GREEN |
| integration_testing | 920724a3 | Add VM integration tests |
| completed | (main task) | Feature complete |

## Usage

### For OpenCode Agent

When working on this feature:

1. **Before any file operation:**
   ```
   - Check which task is currently active
   - Look up corresponding workflow state in mapping above
   - Verify operation is allowed by that state's policy
   - If not allowed, refuse and explain why
   ```

2. **When completing a task:**
   ```bash
   engram task update <task-id> --status done
   # Then move to next task in sequence
   ```

3. **When transitioning states:**
   ```bash
   # Currently manual due to engram limitation
   # Just update task status and track state manually
   ```

### For Humans

To check current workflow status:

```bash
# View workflow definition
engram workflow get b90ee9ad-2d1e-4f17-ba78-06d980fc75b8

# View workflow instance status  
engram workflow status e5f2e4dc-beb3-411a-a443-6257f5b452a7

# List all tasks
engram task list | grep -E "(75aef678|c3c4d6c8|0390c634|c2aa22fd|ce02255a|920724a3)"
```

To manually transition (if needed):

```bash
# When engram supports it in future:
engram workflow transition <instance-id> --transition <transition-name> --agent default
```

## Benefits of This Approach

1. **Enforced Discipline**: Can't accidentally commit code during planning phase
2. **Clear Progress Tracking**: Always know which BDD phase you're in
3. **Test-First Development**: RED phase forces writing failing tests first
4. **Quality Gates**: Each transition has validation criteria
5. **Documentation**: Workflow serves as process documentation
6. **Reusable**: Can use this workflow for future BDD features

## Known Limitations

1. **Manual Transitions**: Workflow transitions must be manually executed (engram instances currently stuck in "initial" state)
2. **Agent-Enforced Only**: Commit policies only enforced by OpenCode agent, not by git hooks
3. **No Automatic Triggers**: Transitions don't auto-execute based on task completion
4. **State Tracking**: Must track state based on task mapping, not workflow instance state

## Future Improvements

When engram adds these features:

1. **Automatic Transitions**: Transitions execute when task marked done
2. **Commit Hook Integration**: Git hooks check workflow state
3. **Test Result Triggers**: Transitions require test pass/fail validation
4. **State-Based Policies**: Workflow states have enforceable commit restrictions

---

## Quick Reference

**Current Status Check:**
```bash
engram task list | grep todo | head -1
```

**Current Workflow State:**
Map the active (todo/inprogress) task ID to the table above

**Can I Commit Code?**
- specification: NO
- framework_setup: Config only
- red_phase: Tests only
- green_phase: YES
- refactor_phase: YES (tests must stay green)
- integration_testing: YES (all quality gates must pass)
- completed: YES
