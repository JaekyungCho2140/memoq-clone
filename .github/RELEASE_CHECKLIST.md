# Release Checklist

> **Owner:** QA Manager  
> **Authority:** QA Manager may block any release at any gate.  
> **Usage:** Copy this checklist into each release PR / Staging issue and complete it before proceeding.

---

## Release Gate Overview

```
Gate 1: CI/E2E (automated)  →  Gate 2: Staging (manual QA)  →  Gate 3: Canary (10%)  →  Full Release
```

---

## Gate 1: CI / Automated Tests ✅

_Automated — passes or fails in CI pipeline (`ci.yml`, `test.yml`)._

- [ ] Frontend TypeScript checks pass (`npm run typecheck`)
- [ ] ESLint passes (`npm run lint`)
- [ ] Frontend unit tests pass (`npm test`)
- [ ] `npm audit` reports no high/critical vulnerabilities
- [ ] Rust `cargo fmt` check passes
- [ ] Rust `cargo clippy` passes (zero warnings)
- [ ] Rust unit tests pass (`cargo test`)
- [ ] WASM plugin builds successfully (`example-mt-provider`)
- [ ] `cargo audit` reports no advisories

**Gate 1 result:** `PASS` / `FAIL` (auto-determined by CI)

---

## Gate 2: Staging Validation 🔍

_Manual — QA Manager must sign off before canary deployment._

### Build & Install

- [ ] macOS DMG mounts and installs cleanly
- [ ] Windows NSIS installer runs without errors (silent + interactive)
- [ ] App icon and name appear correctly in OS launcher

### App Launch

- [ ] App launches without crash on macOS (Intel)
- [ ] App launches without crash on macOS (Apple Silicon)
- [ ] App launches without crash on Windows 10
- [ ] App launches without crash on Windows 11
- [ ] Main window renders at correct default size (1280×800)
- [ ] No console errors on startup (Dev Tools)

### Core CAT Functionality

- [ ] New translation project can be created
- [ ] XLIFF file imports correctly — segments appear in editor
- [ ] Segment text is editable (source / target)
- [ ] Segment status transitions work (untranslated → translated → reviewed)
- [ ] XLIFF export generates a valid file (validate with external tool)
- [ ] Keyboard shortcuts work (confirm/next segment, previous segment)

### Translation Memory (TM)

- [ ] TM server connection succeeds (local TM)
- [ ] TM fuzzy match suggestions appear during translation
- [ ] TM concordance search returns results
- [ ] TM entry can be added manually
- [ ] TM server disconnects cleanly; app continues without crash
- [ ] **Offline scenario:** App works with no TM connected (no crash, graceful fallback)

### File Import / Export

- [ ] XLIFF 1.2 import works
- [ ] XLIFF 2.0 import works (if supported)
- [ ] Export file opens in reference tool (memoQ / OmegaT) without errors
- [ ] File with special characters in path imports correctly

### Network Disconnected Scenario

- [ ] Disable network interface → app does not crash
- [ ] Offline TM lookup returns cached/local results or shows clear "offline" indicator
- [ ] Re-enabling network → app reconnects without requiring restart

### Performance

- [ ] App startup time < 5 seconds on baseline hardware
- [ ] Loading a 500-segment XLIFF file completes in < 3 seconds
- [ ] No visible memory leak during 10-minute editing session

### Regression

- [ ] Issues fixed in this release are verified as resolved
- [ ] No previously passing smoke tests now fail

---

### Gate 2 Sign-Off

> QA Manager must complete this section before canary deployment proceeds.

**Staging build:** `v________`  
**Staging date:** `________`  
**macOS tested on:** `macOS ________ (Intel / Apple Silicon)`  
**Windows tested on:** `Windows ________ (10 / 11)`  
**QA Manager:** `________`

**Blockers found:**
- _List any blocking issues here. Link to GitHub issues._

**Gate 2 result:** `PASS` / `BLOCK`

> ⛔ If result is `BLOCK`, the canary workflow must NOT be triggered.  
> Create a GitHub issue for each blocker and tag it `release-blocker`.

---

## Gate 3: Canary Monitoring 📊

_Automated — canary.yml handles deployment and health monitoring._

- [ ] Canary release deployed to ~10% of users
- [ ] Crash rate monitored for 30 minutes
- [ ] Crash rate remains below 1.0% threshold
- [ ] No critical errors reported in telemetry
- [ ] User feedback channel checked (GitHub Discussions / in-app)

**Gate 3 result:** `PROMOTED` / `ROLLED BACK` (auto-determined by canary.yml)

---

## Full Release

- [ ] Stable release published on all channels
- [ ] `latest.json` updater manifest updated
- [ ] In-app auto-update tested from previous version
- [ ] CHANGELOG.md reflects all changes
- [ ] Release notes published on GitHub Releases
- [ ] Team notified (Slack / email)

---

## Post-Release (24h after)

- [ ] No spike in crash reports since full release
- [ ] No blocking issues reported by users
- [ ] Release tagged as stable in analytics dashboard

---

_Template maintained by CTO. QA Manager owns Gate 2 sign-off authority._
