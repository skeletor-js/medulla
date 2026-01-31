# visual design

> terminal modernism. sharp corners. subtle glow.

the design language bridges retro terminal aesthetics with modern usability. it should feel like a beautifully-designed TUI that escaped into the browser.

---

## core principles

### sharp corners only

no rounded corners. ever. this reinforces the terminal/ASCII aesthetic.

```css
border-radius: 0; /* always */
```

### dark mode first

dark mode is the primary experience. light mode is supported but secondary.

### minimal depth

avoid heavy drop shadows. use subtle borders and background shifts to create layers.

### purposeful animation

only animate state changes and feedback. no decorative animations. respect `prefers-reduced-motion`.

---

## borders

### standard border

```css
border: 1px solid var(--color-slate); /* #2d3b38 */
```

### emphasized border (teal)

```css
border: 1px solid var(--color-teal); /* #0d9488 */
```

### double border (important elements)

```css
border: 3px double var(--color-teal);
```

### ASCII borders

for special emphasis, use actual box-drawing characters:

```
┌────────────────────────────────────────┐
│ content here                           │
└────────────────────────────────────────┘

╔════════════════════════════════════════╗
║ important content                      ║
╚════════════════════════════════════════╝
```

---

## surfaces

### cards

```css
.card {
  background: var(--color-graphite); /* #1a2422 */
  border: 1px solid var(--color-slate); /* #2d3b38 */
  border-radius: 0;
  padding: 16px;
}

.card-elevated {
  background: var(--color-graphite);
  border: 1px solid var(--color-teal);
  box-shadow: 0 0 20px rgba(13, 148, 136, 0.1);
}

.card:hover {
  border-color: var(--color-teal-bright);
  box-shadow: 0 0 30px rgba(45, 212, 191, 0.15);
}
```

### terminal blocks

```css
.terminal {
  background: var(--color-void); /* #0a0f0e */
  border: 1px solid var(--color-slate);
  padding: 16px;
  font-family: var(--font-primary);
  font-size: var(--text-code);
}

.terminal:hover {
  box-shadow: 0 0 30px rgba(45, 212, 191, 0.15);
}
```

---

## buttons

### primary button

```css
.button-primary {
  background: transparent;
  color: var(--color-teal-bright); /* #2dd4bf */
  border: 1px solid var(--color-teal-bright);
  padding: 12px 24px;
  font-family: var(--font-primary);
  font-size: 14px;
  letter-spacing: 0.05em;
  cursor: pointer;
  transition: all 100ms ease-out;
  border-radius: 0;
}

.button-primary:hover {
  background: rgba(45, 212, 191, 0.1);
  box-shadow: 0 0 20px rgba(45, 212, 191, 0.2);
}

.button-primary:active {
  background: rgba(45, 212, 191, 0.2);
}
```

### secondary button

```css
.button-secondary {
  background: transparent;
  color: var(--color-mist); /* #b8c9c4 */
  border: 1px solid var(--color-slate); /* #2d3b38 */
  padding: 12px 24px;
  font-family: var(--font-primary);
  font-size: 14px;
  letter-spacing: 0.05em;
  cursor: pointer;
  transition: all 100ms ease-out;
  border-radius: 0;
}

.button-secondary:hover {
  border-color: var(--color-stone);
  color: var(--color-foam);
}
```

### text button / link

```css
.button-text {
  background: transparent;
  color: var(--color-teal-bright);
  border: none;
  padding: 0;
  font-family: var(--font-primary);
  cursor: pointer;
  text-decoration: none;
}

.button-text:hover {
  text-shadow: 0 0 10px rgba(45, 212, 191, 0.5);
}
```

---

## form elements

### text input

```css
.input {
  background: var(--color-void);
  border: 1px solid var(--color-slate);
  color: var(--color-mist);
  padding: 12px 16px;
  font-family: var(--font-primary);
  font-size: var(--text-body);
  border-radius: 0;
  width: 100%;
}

.input:focus {
  outline: none;
  border-color: var(--color-teal);
  box-shadow: 0 0 0 2px rgba(13, 148, 136, 0.4);
}

.input::placeholder {
  color: var(--color-ash);
}
```

---

## animations

### cursor blink

```css
@keyframes blink {
  0%, 50% { opacity: 1; }
  51%, 100% { opacity: 0; }
}

.cursor {
  animation: blink 1s step-end infinite;
}
```

### glow pulse

```css
@keyframes glow-pulse {
  0%, 100% { box-shadow: 0 0 20px rgba(45, 212, 191, 0.2); }
  50% { box-shadow: 0 0 30px rgba(45, 212, 191, 0.4); }
}
```

### transition defaults

```css
/* standard transition */
transition: all 100ms ease-out;

/* for ASCII-style stepped transitions */
transition: all 150ms steps(4);
```

### motion preferences

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

---

## scanlines (optional)

for hero sections or special emphasis, subtle CRT-style scanlines:

```css
.scanlines::after {
  content: '';
  position: absolute;
  inset: 0;
  background: repeating-linear-gradient(
    0deg,
    transparent,
    transparent 2px,
    rgba(0, 0, 0, 0.1) 2px,
    rgba(0, 0, 0, 0.1) 4px
  );
  pointer-events: none;
}
```

use sparingly. this effect works best on large hero areas, not throughout the interface.

---

## iconography

### style

icons should feel like they could be rendered in ASCII. use:

- outline style with 1.5px stroke
- or small block/character elements

### icon libraries

if using a library:

- **Lucide Icons** (outline, 1.5px stroke) — recommended
- **Phosphor Icons** (regular weight)

customize: remove rounded caps/joins, increase stroke weight slightly.

### ASCII icons

```
◆ decision    ┃ task      ░ note      ▸ prompt

▲ high        ─ medium    ▽ low

✓ done        ○ open      ◐ in progress

⟨ ⟩ code      ⌘ command   ↵ enter

› bullet      → arrow     • dot
```

---

## data visualization

### graph styling

- nodes: sharp rectangles with 1px teal border
- edges: straight lines with 90-degree turns (no curves)
- labels: small monospace text
- background: void (#0a0f0e) or carbon (#111918)

### entity type colors

```
decisions:  medulla teal  (#0d9488)
tasks:      signal green  (#34d399)
notes:      stone         (#7a8f8a)
prompts:    signal amber  (#fbbf24)
components: signal blue   (#60a5fa)
links:      signal violet (#a78bfa)
```

### tables

use ASCII box-drawing for tables:

```
┌─────┬───────────┬──────────┬─────────────────────────────┐
│  #  │ id        │ status   │ title                       │
├─────┼───────────┼──────────┼─────────────────────────────┤
│ 001 │ a1b2c3d   │ accepted │ Use Postgres for storage    │
│ 002 │ e4f5g6h   │ accepted │ Authenticate with JWT       │
│ 003 │ i7j8k9l   │ proposed │ Migrate to edge functions   │
└─────┴───────────┴──────────┴─────────────────────────────┘
```

---

## layout

### spacing scale

```css
--space-1: 4px;
--space-2: 8px;
--space-3: 12px;
--space-4: 16px;
--space-5: 24px;
--space-6: 32px;
--space-7: 48px;
--space-8: 64px;
--space-9: 96px;
```

### container widths

```css
--width-sm: 640px;   /* documentation content */
--width-md: 768px;   /* main content */
--width-lg: 1024px;  /* wide content */
--width-xl: 1280px;  /* full-width layouts */
```

### grid

use simple grid layouts. avoid complex multi-column designs that feel too "web 2.0".

---

## page templates

### homepage hero

```
┌─────────────────────────────────────────────────────────────────┐
│  medulla                                      [docs] [github]   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│                                                                 │
│         your project's brain,                                   │
│         accessible to any AI_                                   │
│                                                                 │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ $ medulla init                                            │  │
│  │ ✓ initialized .medulla/ in /projects/myapp                │  │
│  │                                                           │  │
│  │ $ medulla add decision "Use Postgres" --tag=db            │  │
│  │ ✓ decision created: 001 (a1b2c3d)                         │  │
│  │                                                           │  │
│  │ $ medulla search "database"                               │  │
│  │ › 001 Use Postgres for data storage [accepted] [db]       │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
│                    [ get started → ]                            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### documentation page

```
┌─────────────────────────────────────────────────────────────────┐
│  medulla docs                                        [search]   │
├──────────────────┬──────────────────────────────────────────────┤
│                  │                                              │
│  getting started │  # getting started                           │
│  ─────────────── │                                              │
│  › installation  │  install medulla via homebrew or cargo:      │
│    configuration │                                              │
│    first entity  │  ┌─────────────────────────────────────────┐ │
│                  │  │ brew install medulla                    │ │
│  core concepts   │  │ # or                                    │ │
│  ─────────────── │  │ cargo install medulla                   │ │
│    entities      │  └─────────────────────────────────────────┘ │
│    relations     │                                              │
│    search        │  then initialize in your project:            │
│                  │                                              │
│  CLI reference   │  ┌─────────────────────────────────────────┐ │
│  ─────────────── │  │ cd your-project                         │ │
│    medulla init  │  │ medulla init                            │ │
│    medulla add   │  └─────────────────────────────────────────┘ │
│    medulla list  │                                              │
│                  │  ───────────────────────────────────────     │
│                  │                                              │
│                  │  ## next steps                               │
│                  │                                              │
└──────────────────┴──────────────────────────────────────────────┘
```

---

## don'ts

- no rounded corners
- no gradients (except subtle hover effects)
- no drop shadows (use glow effects instead)
- no stock photography
- no illustrations with organic shapes
- no busy patterns or textures
- no more than 2 accent colors per view
- no animations longer than 200ms
- no bouncing or elastic animations
