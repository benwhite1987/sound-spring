# Coding Standards

> A portable baseline for code quality, git workflow, and agent collaboration. Drop this file into the root of any repository and apply.

## Sound Spring adoption

This repository **explicitly adopts** the standards below, enforced via CI (`cargo fmt`, `cargo clippy`, `cargo test`, and `scripts/test-chrome-bindings.sh`).

**Project-specific exceptions:**

- The default branch may remain **`master`**. Renaming to `main` is not required.
- **`AGENTS.md`** is kept locally for agent context and is **excluded from git** (see `.gitignore`).

---

## How to use this document

This document defines three things:

1. **Code quality standards** — how code should be written, regardless of language
2. **Git and version control standards** — how commits, branches, tags, and contributions should be structured
3. **Agent and planning standards** — what files belong in a repo and what doesn't, with specific attention to AI agent context

It is intended to be language-agnostic. Where a section needs language-specific notes (e.g., `gofmt` for Go, `cargo fmt` for Rust), those are called out as examples. Adapt the specifics to your project's stack; keep the principles.

The document is the source of truth. When code or commits drift from it, fix the code or commits — not the document. If the document itself is wrong, change it deliberately, with a commit that explains why.

Reviewers, contributors, and AI agents must use this as the checklist when accepting code into the project. "I didn't read the standards" is not a defense for non-compliant work.

The code-quality sections derive from Robert C. Martin's *Clean Code* and the Academind clean-code checklist. The git sections derive from the conventions used by the Linux kernel, OpenStack, and the [Luis Trinidad gist on Git commit best practices](https://gist.github.com/luismts/495d982e8c5b1a0ced4a57cf3d93cf60).

---

## 1. Code quality

### 1.1 Naming

- **Names reveal intent.** A reader should know what a variable holds, what a function does, or what a type represents without reading further. `elapsedMs` not `e`. `currentUser` not `cu`. `parseRequestBody` not `process`.
- **No misleading names.** Don't call something `list` if it's a map. Don't call a function `getX` if it has side effects.
- **Meaningful distinctions.** `userA` and `userB` is noise. If two things truly differ, name them after that difference (`activeUser`, `pendingUser`).
- **Pronounceable and searchable.** `genTxnRpt` is not pronounceable. Single-letter names are not searchable. Exception: loop counters (`i`, `j`) and well-established short conventions (`fd` for file descriptor, `err` for error in Go, `ev` for event, `ctx` for context).
- **Variables and properties are nouns.** `socketPath`, `requestBody`, `userList`.
- **Functions and methods are verbs or verb phrases.** `loadProfile`, `emitEvent`, `validate`. Not `profile` or `event`.
- **Types, structs, classes are nouns.** `User`, `RequestHandler`, `Connection`.
- **One word per concept.** Pick `load` or `read` or `open` and use it consistently for the same operation across the codebase. Don't mix `fetchUser`, `loadUser`, `readUser` for the same action.
- **No type-encoded prefixes.** `strName` and `iCount` are Hungarian notation; the language already tells you the type. Exception: in C or other typeless contexts, a suffix can disambiguate genuinely different representations (`pathStr` vs `pathBuf`).
- **Avoid generic filler.** `data`, `info`, `manager`, `handler`, `processor`, `helper`, `util` are usually a sign you haven't decided what the thing is. Be specific or refactor until you can be.

### 1.2 Functions

- **Small.** Functions should be small. Then smaller. Twenty lines is a soft ceiling; if you exceed it, ask whether you're doing more than one thing.
- **Do one thing.** A function does one thing if you cannot meaningfully extract another function from it whose name is not just a restatement of its body.
- **One level of abstraction per function.** Don't mix low-level byte manipulation with high-level business logic in the same function. Extract the lower level into its own helper.
- **Few arguments.** Zero is best. One is fine. Two requires a reason. Three or more is a signal to introduce a struct, options object, or builder — or to refactor. Boolean flag arguments are especially bad — they almost always mean the function does two things; split it.
- **No side effects beyond the obvious.** A function named `parseRequest` must not also mutate global state or write a file. If it does both, the name is lying. Split it, or rename to expose what it really does (`parseRequestAndCache`).
- **Command-query separation.** A function either *does* something (changes state) or *answers* something (returns a value). Mixing both — a getter that also mutates — produces bugs that are very hard to spot in review.
- **DRY (Don't Repeat Yourself).** If the same logic appears in two places, extract it. The exception is genuine coincidence: two functions that happen to share a few lines but conceptually do different things. Don't force-merge coincidence into a shared abstraction.

### 1.3 Comments

- **Comments don't make up for bad code.** If you feel the need to explain what a block does, the block needs a better name or a smaller scope, not a comment.
- **Good comments:**
  - Legal/license headers
  - Explanations of *why*, not *what* (the code shows the what)
  - Warnings of consequences (`// this lock must be released before calling X or we deadlock`)
  - TODO markers with enough context to act on later
  - Public API documentation (godoc, JSDoc, rustdoc, docstrings, doxygen)
  - References to external specifications (RFC numbers, ticket IDs, vendor docs)
- **Bad comments — delete on sight:**
  - Restating what the code already says (`i++ // increment i`)
  - Commented-out code (use git history instead)
  - Journal or changelog comments inside source (also git's job)
  - Closing-brace labels (`} // end of function`)
  - Position markers (`/////// SECTION ///////`)
  - Out-of-date comments — actively misleading, delete first
- **Comments must stay close to what they describe.** A comment at the top of a file about a function 200 lines down will drift and rot.

### 1.4 Formatting

- **Vertical openness separates concepts.** Blank line between related groups of lines. No blank lines inside a tight group.
- **Vertical density keeps related lines close.** Variable declaration immediately above its first use, not at the top of a 100-line function.
- **Vertical distance: related things stay near each other.** Caller above callee where possible (top-down reading order, "newspaper" structure — headline at the top, details below).
- **Horizontal width: ~100 columns soft limit, 120 hard.** If a line is longer, break it. Long lines force horizontal scrolling, which kills review quality.
- **Indentation reflects scope.** Don't fight the language's natural indentation.
- **Match the language's idiomatic style.** Run the canonical formatter as a mandatory pre-commit step: `gofmt` for Go, `cargo fmt` for Rust, `black` for Python, `prettier` for TS/JS/CSS/HTML, `rustfmt` for Rust, `clang-format` for C/C++ (with a project `.clang-format`), `gleam format` for Gleam, etc. No exceptions: formatter output is the project's canonical formatting.

### 1.5 Control flow and errors

- **Prefer positive conditions.** `if isReady` reads better than `if !isNotReady`. Double negatives are a code smell.
- **Guard clauses, not deep nesting.** Return early on invalid input. The "happy path" should run down the left margin of the function with minimal indentation.
  ```
  // bad
  if (user != null) {
      if (user.active) {
          if (user.hasPermission()) {
              // ... real work ...
          }
      }
  }
  // good
  if (user == null)        return error("no user");
  if (!user.active)        return error("inactive user");
  if (!user.hasPermission()) return error("permission denied");
  // ... real work ...
  ```
- **Extract complex conditions into named functions or named booleans.** `if (isExpiredSession(s))` beats `if (s.lastSeen + s.ttl < now && s.state != STATE_REFRESHING)`.
- **Handle errors at the right layer.** Don't blindly propagate errors through five layers — wrap with context so the error message tells the user where it broke. In Go: `fmt.Errorf("loading config %s: %w", path, err)`. In Rust: `.context("loading config")?`. In Python: `raise ConfigError(...) from err`.
- **Check every fallible call.** I/O, syscalls, network, parsing — anything that can fail. Either handle the failure or propagate it with context. Never silently ignore.
- **No synthetic errors for control flow.** Don't throw/panic to control normal flow. Errors are for unexpected conditions, not "the user clicked cancel."
- **Prefer exceptions to error codes** in languages that support them well (Python, Java, C#, JS). Prefer error values in languages that lean that way (Go, Rust). Match the language's idiom; don't import a paradigm from elsewhere.

### 1.6 Structure (types, modules, packages)

- **Single responsibility.** A type, file, or package should have one reason to change. A `UserRepository` reads and writes users; it doesn't also send emails or generate reports.
- **High cohesion.** Things that change together live together. A parser, its grammar definitions, and its types belong in one package; they don't belong split across `parser/`, `grammar/`, and `types/`.
- **Low coupling.** Modules depend on as few other modules as possible, and only on stable interfaces. Public API surface is small and intentional; internals stay internal.
- **Encapsulate state.** In Go, lowercase field names by default; export only what callers actually need. In Rust, `pub(crate)` and `pub(super)` exist for a reason. In Python, leading underscore signals "internal." In C, prefer `static` for module-local functions and globals. In TypeScript, `export` only what's part of the module's public contract.
- **Law of Demeter.** Don't reach through chains of structures (`a.b.c.d.doSomething()`). Talk to your immediate collaborator; let it talk to its collaborator. Chained access is a sign of leaky abstractions.

### 1.7 Tests

- **Every non-trivial function should be testable.** Pure functions are trivially testable; functions that touch the kernel, filesystem, network, or clock need either dependency injection or a clear integration-test boundary.
- **Test names describe behavior, not implementation.** `TestUserCannotLoginWithExpiredToken`, not `TestLogin3`.
- **Tests must be fast.** Unit tests run in milliseconds. Integration tests run separately, gated behind a build tag (`-tags integration` in Go), a separate test suite (`pytest -m integration`), or a make target (`make integration-test`).
- **One concept per test.** A test that exercises three unrelated behaviors will fail in confusing ways. Split it.
- **Test the public interface.** Don't test private helpers directly unless they're complex enough to warrant it — usually that means they should be public, or should be tested through the public surface that calls them.
- **No flakes.** A test that fails intermittently is worse than no test: it trains the team to ignore failures. Fix the flake or delete the test. Common sources: time/clock dependencies, network calls without mocks, shared state between tests, parallel-test ordering assumptions.

### 1.8 What an agent must do (code)

When an AI agent implements any task in this project:

1. **Read this document before writing code** for any module. The check is per-task, not per-session.
2. **Apply these standards at write time, not in a cleanup pass.** Naming, function size, error handling — these are decisions made as code is written, not retrofitted.
3. **Match existing project style.** If the project has a dominant style (formatting, naming patterns, error-handling conventions), match it exactly. Don't introduce a new style mid-project. When unsure, grep for examples of the relevant pattern in the existing codebase before writing new code.
4. **Run language formatters before declaring work done.** Whatever the canonical formatter for the language is (see §1.4), running it is non-negotiable. No commit without a clean formatter pass.
5. **Run linters before declaring work done.** `golangci-lint` for Go, `clippy` for Rust, `ruff` or `flake8` for Python, `eslint` for TS/JS. If the project has a linter config, respect it. If it doesn't and the language has an obvious default, suggest adding one.
6. **Flag violations of these rules in any code being modified, even if the violation predates the task.** Don't silently propagate bad patterns. Either fix them as part of the task (small fix) or call them out and ask whether to address them separately (larger fix).
7. **Never disable a test, lint, or check to make a task "pass."** If a check is wrong, fix the check explicitly with a commit that explains why. If a test is wrong, fix the test. Suppressing a check to clear a CI run is dishonest engineering.

---

## 2. Git and version control

All repositories follow the conventions in this section. The goal is a git history that a maintainer or downstream packager can read, understand, and bisect months or years from now without context from the original author.

### 2.1 Commit hygiene

- **One logical change per commit.** Fixing two bugs → two commits. Adding a feature plus refactoring the file it touches → two commits (refactor first, then the feature on top, so the refactor is reviewable in isolation). The bar: each commit should leave the tree in a buildable, testable state and have a defensible one-sentence summary.
- **Commit often.** Small commits are easier to review, easier to revert, easier to bisect when something breaks weeks later. Resist the urge to roll up a day's work into one commit at end-of-day.
- **Don't commit half-done work.** If you need a clean tree to switch branches or pull, use `git stash`. The only thing in the repo's history is finished, reviewable work.
- **Test before committing.** Build it. Run the unit tests. Don't commit code you "think" works. The shorter the path between "I wrote it" and "I verified it," the fewer broken commits land.
- **Never amend a published commit.** Once a commit is pushed to a public remote or sent as a patch, it is immutable. Make a follow-up commit. `git commit --amend` is for fixing the *most recent local* commit only.

### 2.2 Commit message format

Strict 50/72 rule, imperative present tense. This matches what `git merge` and `git revert` auto-generate, and what nearly every well-maintained open-source project uses.

```
<Capitalized 50-char-max summary in imperative present tense>

<Optional body, wrapped at 72 characters. Explains the WHY,
not the WHAT — the diff already shows the what. Reference
specific failure modes, ticket numbers, or design decisions
when helpful. Separated from the summary by exactly one blank
line.>

<Optional further paragraphs after a blank line.>

- Bullet points are okay
- Use a hyphen, single space, hanging indent on continuation
- Blank line between bullets when bullets are paragraphs

<Optional trailers at the end, in Lower-Case-Hyphenated form:>
Signed-off-by: Name <email>
Fixes: a1b2c3d4 ("Brief summary of the commit being fixed")
Refs: #123
```

**Subject line rules:**
- 50 characters or fewer (hard limit: 72)
- Capitalized first letter
- No trailing period
- Imperative present tense: `Add`, `Fix`, `Remove`, `Refactor`, `Document` — never `Added`, `Fixes`, `Removing`
- Reads naturally after "If applied, this commit will…"

**Body rules:**
- Wrap at 72 columns
- Blank line between subject and body — non-negotiable, breaks rebase/email tooling
- Explain motivation and any non-obvious implementation choices
- Reference issue numbers, design docs, or specifications when relevant

**Example 1** — small fix, no body needed:
```
Fix null pointer on session timeout
```

**Example 2** — small feature with bullet-point body:
```
Add retry logic to HTTP client

- Exponential backoff with jitter, starting at 100ms
- Maximum 3 retries before failing the request
- Retries only on 5xx and network errors, not 4xx
- Adds metric for retry count per endpoint
```

**Example 3** — substantive change with explanatory body:
```
Switch background job runner to bounded queue

The previous unbounded queue allowed memory growth to fill
available RAM during traffic spikes, causing OOM kills and
losing all queued work. Production incidents on Sept 12 and
Sept 19 both traced to this.

Replace with a bounded queue (default 10,000 items) plus
backpressure: producers block on Put() when full rather than
silently dropping. Configurable per-runner via WithQueueSize.

Tested by simulating 100k items/sec with WithQueueSize(1000);
verified producers slow down rather than the runner crashing.
Memory stays under 50MB throughout the test.

Refs: #847
```

### 2.3 Branching

- **Default branch** is whatever the repository uses today (`master` or `main`). Do not rename an existing default branch unless the maintainers explicitly request it.
- **Feature branches:** `feature/<short-kebab-name>` for additive work — `feature/oauth-login`, `feature/markdown-export`.
- **Bugfix branches:** `fix/<short-kebab-name>` — `fix/null-on-reload`, `fix/race-in-cache-eviction`.
- **Refactor branches:** `refactor/<short-kebab-name>` when the change is restructuring without behavior change.
- **Release branches:** `release/<version>` only when needed for cherry-picks during a long stabilization window. Usually unnecessary; tag directly from the default branch.
- **One branch, one logical effort.** Long-lived feature branches accumulate unrelated work and become unreviewable. If a branch grows past ~10 commits or two weeks, split it.
- **Never push to the default branch directly.** Even for one-line fixes. Always go through a branch and a review (even if the review is "self-review on a quiet PR").
- **Never force-push to a shared branch.** Force-pushing to your own feature branch before review is fine. Force-pushing to a branch others have based work on, or to the default branch, is a blocking error.

### 2.4 Workflow

The platform doesn't matter; the discipline does. The same rules apply whether the project lives on:
- **sr.ht** — patch-based contribution via `git send-email`
- **GitHub / GitLab / Codeberg / Forgejo / Gitea** — pull/merge requests
- **Self-hosted** — whatever review mechanism is set up

#### 2.4.1 Patch-based workflow (sr.ht, kernel-style, mailing lists)

```bash
git config sendemail.to <project-mailing-list>
git send-email --annotate origin/main..HEAD
```

For multi-commit series:
- Include a cover letter (`git send-email --cover-letter`) for series of more than ~3 patches
- Cover letter explains the series as a whole; individual commit messages explain each patch
- Version respins use `-v2`, `-v3` in the subject prefix (`git send-email --subject-prefix='PATCH v2'`)
- Respond to review comments by sending a new version of the patchset, not by replying to the email thread with the new diff

#### 2.4.2 PR-based workflow (GitHub, GitLab, etc.)

- **PR title** follows the same 50-char imperative-present rule as commit subjects
- **PR body** expands on motivation, references linked issues, and describes test results
- **PR commits** stay clean — squash WIP and "fix typo" commits before merging, but preserve meaningful intermediate commits
- **No merge commits on the default branch.** Rebase or squash. Linear history is easier to read and bisect.
- **One PR, one logical change.** Same rule as branches. If a PR's description starts with "this also includes…" — split the PR.
- **Self-review before requesting review from others.** Read your own diff line by line as if you were the reviewer. You'll catch half the problems before anyone else sees them.

### 2.5 Tags and releases

- **Annotated tags only.** `git tag -a v1.0.0 -m "Release v1.0.0"`. Never lightweight tags. Annotated tags carry metadata (tagger, date, message); lightweight tags are just floating pointers that disappear from history easily.
- **Semantic versioning.** `MAJOR.MINOR.PATCH`. Pre-1.0 may use `0.x.y` with looser rules.
- **Tag from the default branch.** If a hotfix is needed off an old release, that's the one case for a `release/v1.0.x` branch.
- **CHANGELOG.md updated in the same commit that bumps the version.** Keep a `## [Unreleased]` section at the top during development; on release, rename `Unreleased` to the new version with a date, and open a fresh `Unreleased` section.
- **Release notes** (sr.ht annotated tags, GitHub Releases, GitLab Releases) link to the relevant CHANGELOG section. Don't duplicate the changelog into release notes — keep one source of truth.

### 2.6 What an agent must do (git)

When an AI agent makes any change to a repository:

1. **Work on a branch, never directly on the default branch.** Branch name per §2.3.
2. **Make one commit per logical change.** If a task involves two separable changes, produce two commits. Don't conflate.
3. **Write commit messages per §2.2.** 50-char subject, imperative present, blank line, 72-wrapped body when a body is warranted. The body must explain motivation, not restate the diff.
4. **Run the full test suite before committing.** Whatever the project's test command is (`make test`, `cargo test`, `go test ./...`, `pytest`, `npm test`), run it. No commit without a green test run, unless the commit is *making* a test green and that's the explicit purpose.
5. **Never amend or rebase commits that have been pushed or sent as patches.** If a fix is needed, add a new commit.
6. **For multi-commit series, write a brief summary** in the cover letter or PR description. Write it from the perspective of a maintainer reviewing the series, not the author writing it.
7. **Update CHANGELOG.md in the same commit as user-visible changes.** Don't defer changelog updates to "later" — they get forgotten.
8. **Sign off commits with `Signed-off-by:`** when contributing to upstream projects that require DCO (Developer Certificate of Origin). For the project's own repos, sign-off is optional but recommended.
9. **Never push to the default branch directly. Never force-push to a shared branch.** Both are blocking errors; if the agent finds itself wanting to do either, stop and ask the human first.
10. **Never auto-resolve a merge conflict the agent doesn't understand.** When uncertain about how to merge, stop and surface the conflict to the human. A wrong merge is far worse than a delayed merge.

---

## 3. Excluded files

Some files exist for development, planning, and AI-assisted workflows but are **never committed** to public repositories. This section defines what to exclude and why.

### 3.1 .gitignore baseline

The following patterns should appear in **every repository's `.gitignore`**. Treat the list as a baseline; individual repos may add more (project-specific build artifacts, generated assets, etc.).

```gitignore
# ──────────────────────────────────────────────────────────
# AI agent context and prompts — never committed
# ──────────────────────────────────────────────────────────
CLAUDE.md
AGENTS.md
GEMINI.md
.claude/
.cursor/
.cursorrules
.aider*
.copilot/
.github/copilot-*
.continue/
.windsurf/
.codeium/

# AI session artifacts
transcripts/
conversation-*.md
agent-notes.md
*.agent.md
*.session.md

# ──────────────────────────────────────────────────────────
# Planning and design documents that live outside the repo
# ──────────────────────────────────────────────────────────
planning/
design-notes/
scratch/
draft/
TODO.local.md
NOTES.local.md
*-design.md
*-design-v*.md
*-spec-draft.md

# ──────────────────────────────────────────────────────────
# Local IDE configs that aren't team-wide
# ──────────────────────────────────────────────────────────
.vscode/
.idea/
*.code-workspace
.envrc
.direnv/
.tool-versions.local

# Note: if your team has agreed on a shared .vscode/ config, commit it
# explicitly and remove this line. The default is "personal config stays local."

# ──────────────────────────────────────────────────────────
# Build artifacts (extend per your project's stack)
# ──────────────────────────────────────────────────────────
*.o
*.so
*.a
*.dylib
*.exe
*.dll
build/
dist/
target/
out/
bin/
__pycache__/
*.pyc
*.pyo
.pytest_cache/
node_modules/
.next/
.nuxt/
.svelte-kit/
coverage/
*.log

# ──────────────────────────────────────────────────────────
# OS junk
# ──────────────────────────────────────────────────────────
.DS_Store
Thumbs.db
desktop.ini
*.swp
*.swo
*~
.~lock.*

# ──────────────────────────────────────────────────────────
# Secrets — should never be in a repo, but defense in depth
# ──────────────────────────────────────────────────────────
.env
.env.local
.env.*.local
secrets/
*.key
*.pem
*.p12
*.pfx
credentials.json
service-account.json
```

### 3.2 Why this matters for agents specifically

Agent context files (`CLAUDE.md`, `.claude/instructions`, `AGENTS.md`, AI session notes) are often the most informative documents about a project. They contain decisions, rationale, design context, and instructions that nobody else has written down in such concentrated form. **They are also private.**

These files reflect the human's working style and may:
- Contain unfinished thoughts or speculative directions
- Reference unrelated projects or personal context
- Include exploratory ideas the human doesn't want public
- Document AI-assisted development methodology in ways the human may not want publicly associated with the project
- Include prompts and instructions that are useful to the human's workflow but irrelevant or confusing to downstream readers

Committing them to a public repo would (a) confuse downstream readers about what the actual project is vs. what was being planned, (b) couple the project's history to internal planning documents that will continue to evolve separately, and (c) leak working methodology that's not part of the project's deliverable.

**The rule for agents is unconditional: if a file is part of how the project is being built rather than what is being shipped, it does not go in any repo.** When in doubt, leave it out and ask. Adding a file later is trivial; removing a file from public git history is permanent and embarrassing.

This includes design specifications that the agent is referencing. The spec is not the deliverable; the code is the deliverable. The spec lives outside the repo as a planning artifact.

### 3.3 The planning directory

All planning artifacts — design docs, future revisions, notes, scratch work, agent context — live in a **separate, local-only directory**, not a repo. The conventional location is parallel to the working clones:

```
~/projects/<project>-planning/        # local only, never a git repo
├── design.md
├── design-v2.md
├── notes.md
├── todo.md
├── CLAUDE.md                         # agent context lives here, not in any repo
└── transcripts/

~/projects/<project>/                 # local clone of the main repo
~/projects/<project>-companion/       # local clone of any sibling repo
```

If version control is desired for the planning content, initialize it as a **private** repo on a personal account, or a local-only git (`git init` with no remote), but keep it strictly separate from the public project repos.

**Agents must never `git init` inside the planning directory unless explicitly asked to.** Agents must never push planning content anywhere unless explicitly asked to. The default is: planning content stays local, on disk, untracked.

### 3.4 What an agent must do (excluded files)

1. **Before adding any file to a repo, check whether it belongs.** Apply §3.1 patterns. If a file matches an excluded pattern but the agent thinks it should be committed anyway, stop and ask the human first.
2. **Never move a file from the planning directory into a repo without explicit permission.** Even if the file would be useful as project documentation, the human's intent for that file may be different.
3. **Never edit `.gitignore` to *remove* exclusion patterns** without explicit permission. Adding patterns is fine; removing them needs human sign-off.
4. **If asked to "commit everything," interpret that as "commit everything that should be committed."** Apply the exclusion rules. Surface excluded files in the response: "These files were excluded per the standards: …. Should any of them be included?"
5. **Treat the existence of agent-context files in the working directory as load-bearing for your task, but invisible to git.** The agent reads them; the repo doesn't see them.

---

## Appendix: Quick reference

### A. Commit message template

Drop this in `.gitmessage.txt` and configure with `git config commit.template .gitmessage.txt`:

```
# <Capitalized 50-char summary, imperative present tense>
#
# <Optional body, wrapped at 72 chars. Explains WHY, not WHAT.>
#
# <Trailers at the end, e.g.:>
# Signed-off-by: Name <email>
# Refs: #123
```

### B. Pre-commit checklist

Before every commit:

- [ ] Code formatted (`gofmt`, `cargo fmt`, `black`, `prettier`, etc.)
- [ ] Linter clean (`golangci-lint`, `clippy`, `ruff`, `eslint`, etc.)
- [ ] Tests pass (`make test`, `cargo test`, `go test ./...`, `pytest`, etc.)
- [ ] Commit message follows 50/72 rule, imperative present tense
- [ ] Commit contains one logical change
- [ ] CHANGELOG.md updated if user-visible
- [ ] No excluded files staged (`git status` clean of agent/planning artifacts)

### C. Agent self-check before declaring a task done

- [ ] Standards in §1 applied to all new code
- [ ] Standards in §2 applied to all commits
- [ ] No files matching §3.1 patterns committed
- [ ] Tests written for new functionality
- [ ] Tests run and passing
- [ ] Formatter and linter run and clean
- [ ] CHANGELOG.md updated where appropriate
- [ ] No pre-existing violations silently propagated; any found were flagged
- [ ] Branch name follows §2.3 conventions
- [ ] Commit message(s) follow §2.2 conventions

---

*This document is a living standard. Propose changes via the same git workflow it specifies.*
