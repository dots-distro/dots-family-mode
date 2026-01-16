# DOTS Family GUI Build Fix Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix compilation errors in `dots-family-gui` to reach a successful build state for the prototype.

**Architecture:** Rust with Relm4 v0.8.1 and Libadwaita. Following the Component pattern for UI elements.

**Tech Stack:** Rust, GTK4, Libadwaita, Relm4

## Task 1: Implement `Default` for `Profile` in `common`

**Files:**
- Modify: `crates/dots-family-common/src/types.rs`
- Test: `crates/dots-family-common/src/types.rs` (add test)

**Step 1: Write the failing test**

In `crates/dots-family-common/src/types.rs`:

```rust
#[test]
fn test_profile_default() {
    let profile = Profile::default();
    assert!(!profile.active);
    assert_eq!(profile.name, "");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p dots-family-common`
Expected: FAIL (method not found)

**Step 3: Implement `Default` for `Profile`**

In `crates/dots-family-common/src/types.rs`:

```rust
impl Default for Profile {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::new(),
            age_group: AgeGroup::EarlyElementary,
            birthday: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            config: ProfileConfig::default(),
            active: false,
        }
    }
}

// Also need Default for ProfileConfig and sub-structs
impl Default for ProfileConfig {
    fn default() -> Self {
        Self {
            screen_time: ScreenTimeConfig::default(),
            applications: ApplicationConfig::default(),
            web_filtering: WebFilteringConfig::default(),
        }
    }
}
// ... implement Default for other structs
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p dots-family-common`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/dots-family-common/src/types.rs
git commit -m "feat(common): implement Default for Profile types"
```

## Task 2: Fix `app.rs` Syntax Errors

**Files:**
- Modify: `crates/dots-family-gui/src/app.rs`

**Step 1: Fix `view!` macro syntax**

The error `expected identifier` at `[view]` indicates incorrect attribute placement or syntax in the macro. Remove the square brackets around `view` if it's being used as an attribute on an impl block (it shouldn't be, `relm4::component` handles it), or fix the macro invocation.

Looking at the error:
```
92 | ...                   [view]
   |                       ^
```
It seems `#[view]` attribute is being used inside a macro where it shouldn't be, or `view!` macro body has syntax errors.

In Relm4 0.8, the `view!` macro is used inside the `impl SimpleComponent`.

Also fix the missing comma error:
```
98 |                         },
   |                         ^
```

**Step 2: Verify syntax fix**

Run: `cargo check -p dots-family-gui`
Expected: Syntax errors gone, likely still type errors.

**Step 3: Commit**

```bash
git add crates/dots-family-gui/src/app.rs
git commit -m "fix(gui): correct Relm4 view macro syntax in app.rs"
```

## Task 3: Fix `FactoryVecDeque` Builder Pattern

**Files:**
- Modify: `crates/dots-family-gui/src/app.rs`

**Step 1: Update Builder Call**

Change:
```rust
let mut sidebar_rows = FactoryVecDeque::builder(gtk4::ListBox::default())
    .launch()
```
To:
```rust
let mut sidebar_rows = FactoryVecDeque::builder()
    .launch(gtk4::ListBox::default())
```

**Step 2: Verify fix**

Run: `cargo check -p dots-family-gui`
Expected: Arguments error gone.

**Step 3: Commit**

```bash
git add crates/dots-family-gui/src/app.rs
git commit -m "fix(gui): update FactoryVecDeque builder usage for Relm4 0.8"
```

## Task 4: Fix `ProfileEditor` Communication

**Files:**
- Modify: `crates/dots-family-gui/src/views/profile_editor.rs`
- Modify: `crates/dots-family-gui/src/app.rs`

**Step 1: Add `Reset` message to `ProfileEditor`**

In `profile_editor.rs`, add `Reset(Profile, bool)` to `ProfileEditorMsg`.
Handle it in `update` to replace the model state.

**Step 2: Update `app.rs` to send `Reset` message**

In `app.rs`, instead of sending `(Profile, bool)` tuple to the input channel (which expects `ProfileEditorMsg`), send `ProfileEditorMsg::Reset(profile, is_new)`.

**Step 3: Verify fix**

Run: `cargo check -p dots-family-gui`
Expected: Type mismatch error gone.

**Step 4: Commit**

```bash
git add crates/dots-family-gui/src/views/profile_editor.rs crates/dots-family-gui/src/app.rs
git commit -m "fix(gui): use Reset message for ProfileEditor updates"
```

## Task 5: Verify Full Build

**Step 1: Build Workspace**

Run: `cargo build --workspace`
Expected: Success

**Step 2: Commit**

```bash
git commit --allow-empty -m "build: workspace compiles successfully"
```
