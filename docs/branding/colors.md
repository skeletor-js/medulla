# colors

> no pure black. no pure white. everything has subtle warmth.

the palette draws from CRT phosphors, terminal emulators, and the glow of text on dark screens. colors should feel like they emit light rather than reflect it.

---

## primary palette

the teal family is the signature color of medulla.

| name | hex | rgb | usage |
|------|-----|-----|-------|
| **medulla teal** | `#0d9488` | 13, 148, 136 | primary brand color, key actions, links |
| **bright teal** | `#2dd4bf` | 45, 212, 191 | hover states, highlights, success |
| **pale teal** | `#99f6e4` | 153, 246, 228 | light accents, glows, selections |
| **deep teal** | `#134e4a` | 19, 78, 74 | dark accents, pressed states |

---

## neutral palette (warm carbon)

instead of gray, we use warm carbon tones—off-blacks and off-whites with subtle teal undertones.

| name | hex | rgb | usage |
|------|-----|-----|-------|
| **void** | `#0a0f0e` | 10, 15, 14 | deepest background |
| **carbon** | `#111918` | 17, 25, 24 | primary dark background |
| **graphite** | `#1a2422` | 26, 36, 34 | elevated surfaces, cards |
| **slate** | `#2d3b38` | 45, 59, 56 | borders, dividers |
| **ash** | `#4a5c58` | 74, 92, 88 | muted text, placeholders |
| **stone** | `#7a8f8a` | 122, 143, 138 | secondary text |
| **mist** | `#b8c9c4` | 184, 201, 196 | body text on dark |
| **cloud** | `#d4e4df` | 212, 228, 223 | primary text on dark |
| **foam** | `#e8f4f0` | 232, 244, 240 | headings on dark, light bg |
| **chalk** | `#f4faf8` | 244, 250, 248 | lightest background |

---

## semantic palette

| name | hex | usage |
|------|-----|-------|
| **signal green** | `#34d399` | success, completion, accepted |
| **signal amber** | `#fbbf24` | warnings, attention, proposed |
| **signal red** | `#f87171` | errors, destructive, deprecated |
| **signal blue** | `#60a5fa` | information, links (alternate) |
| **signal violet** | `#a78bfa` | special states, experimental |

---

## color application

### dark mode (primary)

```
background:     carbon (#111918)
surface:        graphite (#1a2422)
border:         slate (#2d3b38)
muted text:     stone (#7a8f8a)
body text:      mist (#b8c9c4)
headings:       foam (#e8f4f0)
primary accent: medulla teal (#0d9488)
highlight:      bright teal (#2dd4bf)
```

### light mode

```
background:     chalk (#f4faf8)
surface:        foam (#e8f4f0)
border:         cloud (#d4e4df)
muted text:     ash (#4a5c58)
body text:      graphite (#1a2422)
headings:       carbon (#111918)
primary accent: medulla teal (#0d9488)
highlight:      deep teal (#134e4a)
```

---

## entity type colors

use these colors consistently when representing different entity types:

```
decisions:  medulla teal  (#0d9488)  ◆
tasks:      signal green  (#34d399)  ┃
notes:      stone         (#7a8f8a)  ░
prompts:    signal amber  (#fbbf24)  ▸
components: signal blue   (#60a5fa)  ◈
links:      signal violet (#a78bfa)  ◇
```

---

## glow effects

for emphasis and focus states, use teal glow effects:

```css
/* subtle glow */
box-shadow: 0 0 20px rgba(13, 148, 136, 0.15);

/* focus glow */
box-shadow: 0 0 0 2px rgba(13, 148, 136, 0.4);

/* intense glow (hover, active) */
box-shadow: 0 0 30px rgba(45, 212, 191, 0.25);

/* text glow (use sparingly) */
text-shadow: 0 0 10px rgba(45, 212, 191, 0.5);
```

---

## CSS tokens

```css
:root {
  /* primary */
  --color-teal: #0d9488;
  --color-teal-bright: #2dd4bf;
  --color-teal-pale: #99f6e4;
  --color-teal-deep: #134e4a;
  
  /* neutrals */
  --color-void: #0a0f0e;
  --color-carbon: #111918;
  --color-graphite: #1a2422;
  --color-slate: #2d3b38;
  --color-ash: #4a5c58;
  --color-stone: #7a8f8a;
  --color-mist: #b8c9c4;
  --color-cloud: #d4e4df;
  --color-foam: #e8f4f0;
  --color-chalk: #f4faf8;
  
  /* semantic */
  --color-success: #34d399;
  --color-warning: #fbbf24;
  --color-error: #f87171;
  --color-info: #60a5fa;
  --color-special: #a78bfa;
  
  /* semantic aliases */
  --color-bg: var(--color-carbon);
  --color-surface: var(--color-graphite);
  --color-border: var(--color-slate);
  --color-text-muted: var(--color-stone);
  --color-text: var(--color-mist);
  --color-text-heading: var(--color-foam);
  --color-accent: var(--color-teal);
  --color-accent-hover: var(--color-teal-bright);
}

/* light mode overrides */
@media (prefers-color-scheme: light) {
  :root {
    --color-bg: var(--color-chalk);
    --color-surface: var(--color-foam);
    --color-border: var(--color-cloud);
    --color-text-muted: var(--color-ash);
    --color-text: var(--color-graphite);
    --color-text-heading: var(--color-carbon);
    --color-accent-hover: var(--color-teal-deep);
  }
}
```

---

## terminal colors (ANSI)

for CLI output, map ANSI colors to the medulla palette:

```
Black (0):          #111918  (carbon)
Red (1):            #f87171  (signal red)
Green (2):          #34d399  (signal green)
Yellow (3):         #fbbf24  (signal amber)
Blue (4):           #60a5fa  (signal blue)
Magenta (5):        #a78bfa  (signal violet)
Cyan (6):           #2dd4bf  (bright teal)
White (7):          #b8c9c4  (mist)

Bright Black (8):   #4a5c58  (ash)
Bright Red (9):     #fca5a5
Bright Green (10):  #6ee7b7
Bright Yellow (11): #fcd34d
Bright Blue (12):   #93c5fd
Bright Magenta (13):#c4b5fd
Bright Cyan (14):   #99f6e4  (pale teal)
Bright White (15):  #e8f4f0  (foam)
```

---

## don'ts

- never use pure black (`#000000`)
- never use pure white (`#ffffff`)
- never use more than two accent colors in one view
- never use teal for error states
- never use low-contrast color combinations
