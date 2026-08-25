# Roadmap

This roadmap captures high-value follow-up work after the issue-complete MVP.
The current product boundary stays simple: `nxc` is a client-side CLI for a
normal Nextcloud account, with safe automation primitives for humans, scripts,
and agents.

Future work can grow into admin/server operations, but those capabilities need a
separate safety model and a clearly marked namespace.

## Product priorities

1. Build trust for agents touching private data.
2. Add recovery paths before adding broader mutation paths.
3. Prefer existing Nextcloud HTTP APIs when they cover the job.
4. Keep admin/server capabilities separate from normal user workflows.
5. Preserve stable JSON, dry-run behavior, confirmation gates, and redaction.

## Near-term user and agent features

### 1. File versions

Agents will edit files. Users need rollback.

Candidate commands:

```bash
nxc files versions /Documents/report.md
nxc files versions download /Documents/report.md <version-id> ./report.old.md
nxc files versions restore /Documents/report.md <version-id> --dry-run
```

Why it matters:

- gives agents a recovery story before and after edits
- supports diff/review workflows
- makes automated file modification safer to trust

Safety requirements:

- restore requires `--dry-run` or `--yes`
- output summarizes version metadata without dumping file contents
- restore writes audit events

### 2. Trashbin restore

Guarded delete is good. Recoverable delete is better.

Candidate commands:

```bash
nxc trash list
nxc trash restore <trash-id> --dry-run
nxc trash delete <trash-id> --dry-run
```

Why it matters:

- pairs naturally with existing guarded file deletion
- gives agents a way to undo mistakes
- helps users inspect deleted items from SSH or automation

Safety requirements:

- permanent trash deletion requires `--yes`
- restore previews destination and conflict behavior
- root or broad cleanup operations remain guarded

### 3. Richer shares

Current public-link support proves the share path. The next step is first-class
support for safer sharing modes and share updates.

Candidate commands:

```bash
nxc shares create /path --user alice
nxc shares create /path --group team
nxc shares update 123 --expire-date 2026-05-01
nxc shares permissions 123 --read true --update false
```

Why it matters:

- user and group shares are often safer than public links
- agents can prepare collaboration workflows without exposing a public URL
- share lifecycle management becomes complete enough for real use

Safety requirements:

- public links remain policy-gated
- permission changes preview before mutation
- output redacts tokens and avoids surprise link disclosure

### 4. Unified and full-text search

Search is one of the most agent-native surfaces in Nextcloud.

Candidate commands:

```bash
nxc search "invoice from march" --type files
nxc search "project notes" --all
nxc search "deck launch card" --type deck
```

Potential backends:

- server unified search when available
- full-text search collections when available
- DAV name search fallback
- optional local text index later, with explicit consent

Visual similarity is a separate local-media feature rather than a server search
backend. It is implemented as an explicit profile-scoped index; it does not
pretend that Unified Search or FullTextSearch can compare image pixels.

Why it matters:

- agents need retrieval before they can help with files and notes
- search reduces expensive tree walking
- full-text search is more useful than filename search for knowledge workflows

Safety requirements:

- snippets and matched text are private data
- results should support summaries and limits
- local indexing requires explicit opt-in and clear storage docs

### Implemented visual media search slice

The first visual-search slice is deliberately narrow and useful for workflows
that need to find a screenshot, still, or video frame in a Nextcloud library:

```bash
nxc --profile personal index build --path /Projects --media all
nxc --profile personal files search-image ./reference-frame.png --path /Projects
nxc --profile personal index update --path /Projects
nxc --profile personal index status
nxc --profile personal index clear --yes
```

Images use local perceptual fingerprints. Videos are sampled with `ffmpeg`, then
the strongest candidates are refined around their coarse match timestamps. The
index is opt-in, profile-scoped, incremental by WebDAV metadata/ETag, and stored
in the local cache. Original media is downloaded only to temporary files while
it is being fingerprinted. Semantic embeddings and natural-language image search
remain future work because they introduce model distribution, CPU/GPU, and
privacy trade-offs that this CLI should not impose by default.

### 5. Tags, favorites, and comments

Metadata turns file access into workflow coordination.

Candidate commands:

```bash
nxc files favorite /Documents/spec.md
nxc files tags add /Documents/spec.md project-x
nxc files tags remove /Documents/spec.md project-x
nxc comments list /Documents/spec.md
nxc comments add /Documents/spec.md "Reviewed by agent"
```

Why it matters:

- agents can mark important files without rewriting them
- comments create a collaboration trail
- tags support project organization across files

Safety requirements:

- comment creation is a write operation
- comments may contain private context
- tag mutation should dry-run when possible

### 6. Notifications

Notifications help agents answer, "what needs attention?"

Candidate commands:

```bash
nxc notifications list
nxc notifications dismiss <id> --dry-run
```

Why it matters:

- useful read-only dashboard surface
- helps scripts and agents triage account state
- complements Activity without scanning broad history

Safety requirements:

- default limits stay small
- notification subjects can contain private filenames or names
- dismissal requires preview or explicit confirmation

## Admin and server features

Admin capabilities are valuable, but they must live behind an admin/server
namespace and separate configuration. A normal `nxc --profile personal` command
should continue to mean user-scoped HTTP API access.

### 7. Admin provisioning API

Prefer HTTP admin APIs before server-shell execution when Nextcloud exposes the
needed capability.

Candidate commands:

```bash
nxc admin users list
nxc admin users create
nxc admin groups list
nxc admin apps list
```

Why it matters:

- fits the existing client-side model better than server-shell commands
- useful for small self-hosted deployments
- gives admins scriptable JSON without SSH assumptions

Safety requirements:

- admin profiles are visibly marked
- mutations require confirmation
- user, group, quota, and app changes are audit logged

### 8. Server health, logs, and diagnostics

Self-hosters need quick diagnostics.

Candidate commands:

```bash
nxc admin health
nxc admin logs tail
nxc admin security-check
```

Why it matters:

- common maintenance questions become scriptable
- agents can gather diagnostic context before suggesting fixes
- health checks pair well with compatibility reports

Safety requirements:

- logs may contain secrets or personal data
- default output should summarize and redact
- raw log output should require explicit flags

### 9. OCC wrapper

`occ` is a server-side administration tool. It should be supported as an admin
feature with a separate server target. Normal user commands remain scoped to
authenticated HTTP API access.

Candidate setup:

```bash
nxc admin target add cloudbox \
  --ssh user@host \
  --nextcloud-path /var/www/nextcloud \
  --web-user www-data
```

Candidate commands:

```bash
nxc admin occ --target cloudbox -- status
nxc admin occ --target cloudbox -- app:list
nxc admin occ --target cloudbox -- config:list --redact
nxc admin occ --target cloudbox -- files:scan --path="/Nicholai/files/Documents" --dry-run
nxc admin occ --target cloudbox -- maintenance:repair --yes
```

Phase 1 should be read-only and diagnostic:

- `status`
- `app:list`
- `config:list --redact`
- command help passthrough

Later phases can allow selected mutation commands:

- `files:scan`
- `maintenance:repair`
- `db:add-missing-indices`
- maintenance mode changes

Safety requirements:

- separate admin target config
- SSH agent support before stored server passwords
- explicit web server user
- clear server path
- redacted output by default
- allowlist for common commands before arbitrary passthrough
- `--yes` for mutation commands
- command, target, and working directory recorded in audit logs

## Larger future work

### 10. Sync and mirror

Sync is powerful and easy to get wrong. It should come after recovery features,
metadata handling, and conflict policy design.

Candidate commands:

```bash
nxc files sync ./notes /Notes --dry-run
nxc files mirror /remote ./local --dry-run
```

Open design questions:

- conflict policy
- delete propagation
- checksum strategy
- ignore files
- partial transfer recovery
- journal format
- local index storage

Safety requirements:

- dry-run first
- clear conflict report
- no default destructive mirror behavior
- local and remote delete propagation opt-in

## Ranking

Recommended implementation order:

1. file versions
2. trashbin restore
3. richer shares
4. unified and full-text search
5. tags, favorites, and comments
6. notifications
7. admin provisioning API
8. server health, logs, and diagnostics
9. OCC wrapper
10. sync and mirror

The trust-building path starts with recovery. Versions and trashbin make agent
file access safer immediately. OCC belongs on the roadmap, but in the admin phase
with a separate server target and stricter guardrails.
