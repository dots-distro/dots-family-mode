# dots-family-common

Core shared types, configurations, and utilities for the DOTS Family Mode parental control system.

## Overview

This crate provides the foundational types and configuration structures used across all DOTS Family Mode components. It includes:

- Profile types with age-based defaults
- Policy enforcement configurations 
- Error handling types
- Security utilities (password hashing, encryption)
- Serialization support via Serde

## Key Types

### Profiles
- `Profile` - Core user profile with settings
- `AgeGroup` - Pre-configured age ranges (5-7, 8-12, 13-17)
- `ProfileConfig` - Complete profile configuration

### Security
- Password hashing with Argon2
- Encryption key derivation
- Session token generation
- Rate limiting utilities

### Configuration
- `ScreenTimeConfig` - Daily/weekend limits and time windows
- `ApplicationConfig` - Allow/block lists and categories
- `WebFilteringConfig` - Content filtering settings
- `TerminalFilteringConfig` - Command-line restrictions

## Usage

```rust
use dots_family_common::{Profile, AgeGroup};

// Create a profile for a child
let profile = Profile::new("alice", AgeGroup::LateElementary);
```

## Features

- Comprehensive error handling with `anyhow`
- Type-safe serialization with `serde`
- Age-appropriate default configurations
- Production-ready security primitives

This crate forms the foundation for all other DOTS Family Mode components.