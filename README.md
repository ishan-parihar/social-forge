# Postiz Rust 🚀

**A Dual-Interface Social Media Scheduling Engine for Humans and AI.**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
![Rust](https://img.shields.io/badge/rust-1.85%2B-dea584?logo=rust)
[![CI](https://github.com/ishanpm/postiz-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/ishanpm/postiz-rust/actions/workflows/ci.yml)
[![MCP](https://img.shields.io/badge/Protocol-MCP-purple.svg)](https://modelcontextprotocol.io/)
[![Axum](https://img.shields.io/badge/Framework-Axum-blue)](https://github.com/tokio-rs/axum)

`Postiz Rust` is a high-performance social media orchestration engine that implements a unique **Dual-Interface Architecture**. It exposes a single, unified business logic layer through two distinct protocols: a **REST API** for human-operated dashboards and an **MCP (Model Context Protocol)** server for AI-driven automation.

By rewriting a complex scheduling platform in Rust, `Postiz Rust` eliminates the "runtime bloat" of traditional JS/Python backends, providing sub-millisecond response times and a tiny memory footprint while maintaining a massive feature set across 5+ social providers.

---

## 🚩 The Problem: The "Interface Silo"

Most social media tools are designed for one of two audiences:
1. **Human-Centric**: Beautiful GUIs, but opaque APIs that make programmatic control difficult.
2. **API-Centric**: Powerful for developers, but lacking the visual orchestration needed for high-level content strategy.

Furthermore, scheduling platforms often struggle with **Execution Reliability**:
- **The "Ghost Post" Problem**: Failures during API calls to social networks often go unnoticed or are lost in logs.
- **Runtime Overhead**: Heavy Node.js/Python runtimes lead to slow startup times and high memory usage in containerized environments.
- **Auth Fragility**: Managing OAuth tokens across multiple providers (Meta, X, LinkedIn) often leads to fragmented, inconsistent auth logic.

## 💡 The Solution: A Unified Orchestration Substrate

`Postiz Rust` breaks the silo by implementing a **Shared AppState Architecture**.

### The Dual-Interface Pipeline
`Request` $\to$ `Protocol Adapter (REST or MCP)` $\to$ `Shared Business Logic` $\to$ `Provider Registry` $\to$ `Social API`

1. **SvelteKit $\to$ REST**: Human users manage their calendar via a high-performance Axum REST API.
2. **AI Agent $\to$ MCP**: AI agents (via Claude/Cursor) schedule, analyze, and oversee posts using 15+ precision MCP tools.
3. **In-Process Scheduler**: A dedicated Tokio-based background worker polls the database and executes due posts with exponential-backoff retry logic.

---

## ✨ Engineering Highlights

### 🛠 Systems Architecture
- **Zero-Duplicate Logic**: Both the REST and MCP interfaces are thin wrappers around the same `AppState`. A change to the `SocialProvider` trait instantly updates both human and AI interfaces.
- **Trait-Based Provider System**: Uses a dynamic `ProviderRegistry` with `#[async_trait]`, allowing new social networks to be added with zero changes to the core engine.
- **Real-Time Event Stream**: Implements **SSE (Server-Sent Events)** via `tokio::sync::broadcast`, allowing the frontend and AI agents to receive live "post-published" or "execution-failed" notifications.
- **Hardened Auth**: Combines **JWT (jsonwebtoken)** for user sessions with **Argon2** password hashing and a multi-account cookie profile manager.

### 🏗 Technical Specifications
- **Language**: Rust (Edition 2021)
- **Web Framework**: Axum 0.8 (High-performance asynchronous routing)
- **Database**: PostgreSQL via `sqlx` (Compile-time checked queries)
- **Scheduler**: Custom in-process `tokio::spawn` loop with 30s polling
- **Deployment**: Static `musl` binaries resulting in a **~15 MB** Docker image.

---

## 🌌 Potentialities & Future Scope

`Postiz Rust` is designed to be the foundation for **Algorithmic Content Distribution**:

- **AI-Driven Scheduling**: Integrating a "Best-Time-to-Post" agent that analyzes engagement data and autonomously shifts the schedule.
- **Multi-Tenant SaaS**: Evolving the `AppState` to support a multi-tenant architecture with partitioned PostgreSQL schemas.
- **Cross-Platform Content Transformation**: Using an LLM to automatically rewrite a single "Master Post" into platform-specific formats (e.g., a long-form LinkedIn post $\to$ a X thread) before scheduling.
- **Predictive Analytics**: Implementing a "Reach Forecaster" that uses historical data to predict post performance before it's published.

---

## 🚀 Quick Start

### Installation
```bash
git clone https://github.com/ishan-parihar/postiz-rust.git
cd postiz-rust
cp .env.example .env
```

### Database Setup
```bash
# Start Postgres via Docker
docker compose up -d postgres
# Run migrations
# (handled automatically by the app on startup or via custom script)
```

### Run the Engine
```bash
# Development mode
cargo watch -x run

# Production build
cargo build --release
./target/release/postiz-rust
```

## 🛠 Tech Stack
- **Language**: Rust
- **Framework**: Axum
- **DB**: PostgreSQL (sqlx)
- **Auth**: JWT / Argon2
- **Real-time**: SSE / Tokio Broadcast
- **Protocol**: MCP (Model Context Protocol)

---
Developed by [Ishan Parihar](https://github.com/ishan-parihar) as an exploration into dual-interface system design and high-performance automation.
