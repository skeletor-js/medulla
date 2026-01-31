# logo

> minimal. terminal-native. unmistakably medulla.

---

## concept: the pathway mark

the logo represents converging neural pathways—multiple streams of information (decisions, tasks, notes) flowing into a unified system. it should feel like ASCII art elevated to iconography.

the mark suggests:

- multiple inputs → unified knowledge
- git branches → clean merge
- distributed → centralized query

---

## design directions

```
option A: converging lines

    ╲   ╱
     ╲ ╱
      │
      │
      ▼


option B: node network

    ●───●
    │ ╲ │
    ●───●


option C: block assembly

    ┌─┐
    │▓│
    └┬┘
     │


option D: bracket frame

    ⟨m⟩


option E: abstract M (pathway-style)

    ╱╲  ╱╲
    ╱  ╲╱  ╲
```

**recommended direction:** converging lines (option A variant)

a minimal mark showing 2-3 lines converging into one. the mark must work at 16×16 pixels (favicon) and scale cleanly.

---

## wordmark

```
m e d u l l a
```

specifications:

- typeface: Doto Bold
- letter-spacing: 0.15em
- always lowercase
- can stand alone or pair with mark

with ASCII frame (optional):

```
┌──────────────────────────────────────┐
│                                      │
│    m e d u l l a                     │
│                                      │
└──────────────────────────────────────┘
```

---

## logo variations

| variation | usage |
|-----------|-------|
| mark only | favicons, small applications, social avatars |
| wordmark only | documentation headers, text-heavy contexts |
| mark + wordmark (horizontal) | primary lockup, website header |
| ASCII-framed wordmark | terminal output, retro contexts |

---

## color versions

| context | mark color | text color | background |
|---------|------------|------------|------------|
| dark mode | bright teal `#2dd4bf` | foam `#e8f4f0` | carbon `#111918` |
| light mode | medulla teal `#0d9488` | carbon `#111918` | chalk `#f4faf8` |
| monochrome dark | foam `#e8f4f0` | foam `#e8f4f0` | carbon `#111918` |
| monochrome light | carbon `#111918` | carbon `#111918` | chalk `#f4faf8` |

---

## clear space

maintain minimum clear space around the logo equal to the height of the "m" in the wordmark (or 1/4 of the mark width if using mark only).

```
        ┌─────────────────────┐
        │                     │
        │   ╲ ╱               │
   ▲    │    │    medulla     │    ▲
   │    │    ▼                │    │
   x    │                     │    x
   │    └─────────────────────┘    │
   ▼◄─x─►                    ◄─x─►▼
   
   x = clear space (1/4 mark width)
```

---

## minimum sizes

| variation | minimum width |
|-----------|---------------|
| mark only | 16px |
| wordmark only | 80px |
| full lockup | 120px |

---

## file formats needed

### mark

- SVG (scalable, primary format)
- PNG @1x, @2x, @4x (raster fallback)
- dark and light versions
- monochrome versions

### wordmark

- SVG
- PNG @1x, @2x
- dark and light versions

### lockup

- SVG
- PNG @1x, @2x
- dark and light versions

### favicon

- ICO (16×16, 32×32 combined)
- PNG 16×16
- PNG 32×32
- PNG 180×180 (Apple touch icon)
- SVG (modern browsers)

### social

- open graph: 1200×630
- twitter card: 1200×600

---

## usage rules

### do

- use provided logo files only
- maintain minimum clear space
- use approved color variations
- scale proportionally
- use on solid backgrounds with sufficient contrast

### don't

- stretch, rotate, or distort
- add effects (shadows, gradients, glows, outlines)
- change colors outside approved palette
- use the mark as a bullet point or generic icon
- place on busy backgrounds
- recreate or redraw the logo
- animate the logo (except subtle glow on hover)
- use the old/alternate versions after a refresh
- add taglines or other text to the logo lockup

---

## logo in context

### website header

```
┌─────────────────────────────────────────────────────────┐
│  [mark] medulla                        docs   github    │
└─────────────────────────────────────────────────────────┘
```

### CLI output

```
$ medulla --version
medulla 0.1.0
```

### documentation

```
# medulla docs

getting started with medulla...
```

### GitHub README

```
        ╲ ╱
         │
         ▼
    m e d u l l a
    
your project's brain, accessible to any AI
```
