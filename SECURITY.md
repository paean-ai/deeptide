# Security Policy

## Reporting a vulnerability

**Please do not report security issues via public GitHub issues.**

Use GitHub's private vulnerability reporting:

> [Report a vulnerability](https://github.com/paean-ai/deeptide/security/advisories/new)

This channel is private; only the maintainers can see it.

## Scope

This repository covers:

- The `deeptide` npm package (a thin redirect to `@paean-ai/zero-cli`).
- The macOS native DeepTide application.
- Documentation in this repository.

For vulnerabilities in the underlying CLI engine, you may also report
upstream at
[a8e-ai/zero-cli](https://github.com/a8e-ai/zero-cli/security/advisories/new).

## Out of scope

- Vulnerabilities in third-party LLM providers (Anthropic, DeepSeek,
  Zhipu, Volcengine, etc.) — please report to those providers directly.
- Issues with user-installed plugins or MCP servers.

## What to expect

We aim to acknowledge reports within 5 business days and to provide a
remediation timeline within 14 days. Coordinated disclosure timelines
will be agreed with the reporter on a case-by-case basis.
