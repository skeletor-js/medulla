# typography

> everything is monospace. no sans-serif.

medulla embraces its terminal heritage. typography should feel like it belongs on a command line—monospaced, precise, and utilitarian. we choose modern monospace fonts optimized for extended reading, not just code.

---

## typefaces

### primary & display: Doto

**Doto** is a modern, modular typeface that perfectly captures the "terminal heritage" of medulla. It is a dot-matrix style font available via Google Fonts.

- unique dot-matrix aesthetic
- highly legible at various sizes
- open source (Google Fonts)
- supports multiple weights for hierarchy

### font stack

```css
--font-primary: "Doto", "Berkeley Mono", "Commit Mono", "JetBrains Mono", 
                "Fira Code", "SF Mono", "Consolas", monospace;

--font-display: "Doto", "Space Mono", "Berkeley Mono", monospace;
```

---

## type scale

all sizes use monospace. the scale is slightly tighter than typical web typography to feel more terminal-native.

| element | size | weight | line height | letter spacing |
|---------|------|--------|-------------|----------------|
| display | 48px / 3rem | bold (Doto) | 1.1 | -0.02em |
| h1 | 32px / 2rem | bold | 1.2 | -0.01em |
| h2 | 24px / 1.5rem | bold | 1.25 | 0 |
| h3 | 20px / 1.25rem | medium | 1.3 | 0 |
| h4 | 16px / 1rem | medium | 1.4 | 0.01em |
| body | 15px / 0.9375rem | regular | 1.6 | 0.01em |
| small | 13px / 0.8125rem | regular | 1.5 | 0.02em |
| caption | 11px / 0.6875rem | medium | 1.4 | 0.03em |
| code | 14px / 0.875rem | regular | 1.5 | 0 |

---

## CSS tokens

```css
:root {
  --font-primary: "Doto", "Berkeley Mono", "Commit Mono", "JetBrains Mono", monospace;
  --font-display: "Doto", "Space Mono", "Berkeley Mono", monospace;
  
  --text-display: 3rem;      /* 48px */
  --text-h1: 2rem;           /* 32px */
  --text-h2: 1.5rem;         /* 24px */
  --text-h3: 1.25rem;        /* 20px */
  --text-h4: 1rem;           /* 16px */
  --text-body: 0.9375rem;    /* 15px */
  --text-small: 0.8125rem;   /* 13px */
  --text-caption: 0.6875rem; /* 11px */
  
  --leading-tight: 1.2;
  --leading-normal: 1.6;
  --leading-relaxed: 1.8;
  
  --tracking-tight: -0.02em;
  --tracking-normal: 0.01em;
  --tracking-wide: 0.05em;
  --tracking-wider: 0.15em;
}
```

---

## headline style

all headlines use sentence case (lowercase except proper nouns):

```
✓ the problem
✗ The Problem

✓ how it works  
✗ How It Works

✓ getting started with medulla
✗ Getting Started With Medulla
```

---

## ASCII typography elements

embrace box-drawing characters and ASCII art for UI elements.

### borders and frames

```
┌─────────────────────────────────────┐
│  medulla: project context engine    │
└─────────────────────────────────────┘

╔═════════════════════════════════════╗
║  IMPORTANT: breaking change         ║
╚═════════════════════════════════════╝

├── decisions/
│   ├── 001-use-postgres.md
│   └── 002-auth-with-jwt.md
└── tasks/
    └── active.md
```

### section dividers

```
─────────────────────────────────────────

════════════════════════════════════════

▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀

░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
```

### progress and status indicators

```
[████████████████████░░░░░░░░] 67%

[■■■■■■■■□□□□] 8/12 complete

◆ accepted   ◇ proposed   ○ deprecated
```

### bullets and lists

```
› item one
› item two
› item three

▸ primary item
  ▹ sub-item
  ▹ sub-item

◆ decision
◇ task  
○ note
```

### tables

```
┌─────┬───────────┬──────────┬─────────────────────────────┐
│  #  │ id        │ status   │ title                       │
├─────┼───────────┼──────────┼─────────────────────────────┤
│ 001 │ a1b2c3d   │ accepted │ Use Postgres for storage    │
│ 002 │ e4f5g6h   │ accepted │ Authenticate with JWT       │
│ 003 │ i7j8k9l   │ proposed │ Migrate to edge functions   │
└─────┴───────────┴──────────┴─────────────────────────────┘
```

### icons and symbols

```
◆ decision    ┃ task      ░ note      ▸ prompt

▲ high        ─ medium    ▽ low

✓ done        ○ open      ◐ in progress

⟨ ⟩ code      ⌘ command   ↵ enter
```

---

## wordmark specifications

the "medulla" wordmark:

- set in Doto (Bold or Black weight)
- letter-spacing: 0.15em (generous, breathable)
- lowercase always
- can be enclosed in ASCII frame for emphasis

```
┌──────────────────────────────────────┐
│                                      │
│    m e d u l l a                     │
│                                      │
└──────────────────────────────────────┘
```

---

## don'ts

- never use sans-serif fonts
- never use serif fonts
- never use title case for headlines
- never capitalize "medulla"
- never use decorative or script fonts
- never use rounded terminals on box-drawing characters
