# medulla brand

> your project's brain, accessible to any AI

this directory contains the brand identity guidelines for medulla. all brand assets, colors, typography, and voice guidelines are documented here.

---

## quick reference

```
┌─────────────────────────────────────────────────────────────────┐
│                     medulla brand reference                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  name          medulla (always lowercase)                       │
│  tagline       your project's brain, accessible to any AI       │
│                                                                 │
│  colors        teal         #0d9488  ████                       │
│                bright teal  #2dd4bf  ████                       │
│                carbon       #111918  ████                       │
│                foam         #e8f4f0  ████                       │
│                                                                 │
│  typography    Doto (primary & display)                         │
│                everything monospace. no sans-serif.             │
│                                                                 │
│  voice         terse. technical. lowercase. no hype.            │
│                                                                 │
│  visual        sharp corners. ASCII borders. subtle glow.       │
│                terminal aesthetic. dark mode first.             │
│                                                                 │
│  don'ts        pure black. pure white. rounded corners.         │
│                exclamation points. emoji. title case.           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## documentation index

| document | description |
|----------|-------------|
| [colors.md](./colors.md) | full color palette with CSS tokens |
| [typography.md](./typography.md) | typefaces, scale, and ASCII elements |
| [logo.md](./logo.md) | logo usage, variations, and rules |
| [voice.md](./voice.md) | verbal identity and writing guidelines |
| [visual.md](./visual.md) | design language, components, and patterns |

---

## brand essence

**"the quiet backbone of project intelligence."**

just as the anatomical medulla oblongata controls vital autonomic functions without conscious effort, medulla the product operates as the always-present, always-syncing knowledge layer that developers rely on without thinking about it.

---

## name treatment

**"medulla" is always lowercase.**

this reflects:

- terminal-native identity (commands are lowercase)
- understated confidence (no need to shout)
- technical authenticity (like `git`, `npm`, `rust`)
- the product's role as foundational infrastructure

usage rules:

- always "medulla" in body text, never "Medulla"
- even at the start of sentences: "medulla stores project decisions..."
- in titles, remains lowercase: "getting started with medulla"
- logo wordmark: lowercase

---

## brand values

| value | expression |
|-------|------------|
| **foundational** | we build infrastructure that disappears into the background while making everything else possible |
| **git-native** | we believe the repository is the source of truth—not another SaaS dashboard |
| **zero-friction** | no accounts, no API keys, no configuration hell—just `medulla init` |
| **AI-agnostic** | we serve all AI tools equally through open protocols |
| **transparent** | human-readable outputs, open source, no black boxes |

---

## brand personality

**archetype: the sage + the craftsman**

medulla embodies the quiet expertise of a master craftsman combined with the deep knowledge of a sage. it doesn't shout—it simply knows and remembers.

personality traits:

- **understated confidence**: knows its value without needing to prove it
- **technically precise**: uses exact language, no marketing fluff
- **warmly competent**: approachable despite depth
- **reliably present**: like a good tool that's always where you left it
- **thoughtfully minimal**: every feature earns its place

voice character: the senior engineer who writes excellent documentation—clear, helpful, occasionally dry-humored, never condescending.

---

## key messages

**primary message (10 words):**

> your project's brain, accessible to any AI tool.

**elevator pitch (30 seconds):**

> medulla is a project context engine that lives in your git repo. it gives AI tools like Claude Code and Cursor structured access to your project's decisions, tasks, and notes. unlike static markdown files, medulla is queryable—ask "what did we decide about authentication?" and get a real answer. it uses CRDTs so it merges cleanly across branches. no API keys, no external services, just `medulla init` and you're done.

**technical positioning:**

> git-native CRDT storage + MCP interface + local embeddings = project memory that actually works.

---

## target audience

**primary: the pragmatic builder**

- solo developers and small team leads (2-10 people)
- values tools that solve real problems without ceremony
- already uses AI coding assistants daily
- frustrated by context loss between sessions
- appreciates good CLI design and Unix philosophy

**secondary: the OSS maintainer**

- maintains one or more open source projects
- struggles with onboarding new contributors
- needs to document decisions without creating overhead
- values git-native solutions
