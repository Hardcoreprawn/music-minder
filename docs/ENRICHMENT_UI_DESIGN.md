# Music Minder UI Design System

## Vision

A **premium, minimal, Winamp-inspired** music player that feels native, fast, and delightful. Every pixel should feel intentional. The UI should get out of the way when you're listening, but empower you when you need control.

### Design Pillars

1. **Audio First** - The music is the star; UI supports, never distracts
2. **Progressive Disclosure** - Simple by default, power when needed
3. **Contextual** - Show relevant options where you are, not in hidden menus
4. **Premium Feel** - Thoughtful animations, consistent spacing, polished details
5. **Winamp DNA** - Compact, efficient, personality without being dated

---

## Color System

### Dark Theme (Primary)

```text
┌─────────────────────────────────────────────────────────────────┐
│  BACKGROUNDS                                                    │
│  ────────────                                                   │
│  Base:         #121215  (near-black, main background)           │
│  Surface:      #1a1a1f  (cards, panels, raised elements)        │
│  Surface-2:    #232328  (elevated surfaces, modals)             │
│  Surface-3:    #2a2a30  (hover states, active items)            │
│                                                                 │
│  BORDERS & DIVIDERS                                             │
│  ──────────────────                                             │
│  Subtle:       #2a2a30  (barely visible separation)             │
│  Default:      #3a3a42  (standard borders)                      │
│  Strong:       #4a4a52  (emphasized borders)                    │
│                                                                 │
│  TEXT                                                           │
│  ────                                                           │
│  Primary:      #f4f4f5  (headings, important text)              │
│  Secondary:    #a1a1aa  (body text, descriptions)               │
│  Muted:        #71717a  (hints, disabled, timestamps)           │
│  Inverse:      #121215  (text on light backgrounds)             │
│                                                                 │
│  ACCENT COLORS                                                  │
│  ─────────────                                                  │
│  Primary:      #6366f1  (indigo - main actions)                 │
│  Primary-Hover:#818cf8  (lighter indigo)                        │
│  Primary-Muted:#4f46e5  (darker indigo, pressed)                │
│                                                                 │
│  Success:      #22c55e  (green - confirmed, playing)            │
│  Warning:      #f59e0b  (amber - needs attention)               │
│  Error:        #ef4444  (red - errors, destructive)             │
│                                                                 │
│  Winamp Green: #00ff00  (sparingly - visualizations, accents)   │
│  Winamp Amber: #ffaa00  (VU meters, warm indicators)            │
└─────────────────────────────────────────────────────────────────┘
```

### Semantic Colors

```text
Playing:        Success (#22c55e) - currently playing track
Selected:       Primary (#6366f1) - selected item
Queued:         Primary-Muted - tracks in queue
Lossless:       Success - high quality indicator
Lossy:          Muted - standard quality
Identifying:    Warning - in progress
Match High:     Success - 90%+ confidence
Match Medium:   Warning - 70-90% confidence
Match Low:      Error - <70% confidence
```

---

## Typography

```text
┌─────────────────────────────────────────────────────────────────┐
│  FONT STACK                                                     │
│  ──────────                                                     │
│  Primary:    "Inter", "SF Pro Display", system-ui, sans-serif   │
│  Mono:       "JetBrains Mono", "SF Mono", monospace             │
│                                                                 │
│  SCALE                                                          │
│  ─────                                                          │
│  Hero:       32px / 700 weight  (Now Playing track title)       │
│  Title:      24px / 600 weight  (Pane headings)                 │
│  Heading:    18px / 600 weight  (Section headings)              │
│  Body:       14px / 400 weight  (Default text)                  │
│  Small:      12px / 400 weight  (Secondary info, metadata)      │
│  Tiny:       10px / 400 weight  (Timestamps, counts)            │
│                                                                 │
│  LINE HEIGHT                                                    │
│  ───────────                                                    │
│  Tight:      1.2  (headings)                                    │
│  Normal:     1.5  (body text)                                   │
│  Relaxed:    1.75 (long-form, descriptions)                     │
└─────────────────────────────────────────────────────────────────┘
```

---

## Spacing System

```text
┌─────────────────────────────────────────────────────────────────┐
│  BASE UNIT: 4px                                                 │
│                                                                 │
│  Space-1:    4px   (tight: icon gaps, inline elements)          │
│  Space-2:    8px   (default: component padding, small gaps)     │
│  Space-3:    12px  (comfortable: between related items)         │
│  Space-4:    16px  (sections: card padding, group separation)   │
│  Space-5:    24px  (major: pane padding, large sections)        │
│  Space-6:    32px  (hero: top-level separation)                 │
│  Space-8:    48px  (massive: page-level spacing)                │
│                                                                 │
│  LAYOUT                                                         │
│  ──────                                                         │
│  Sidebar Width:     200px  (fixed, collapsible to 60px)         │
│  Context Panel:     320px  (right panel, collapsible)           │
│  Player Bar Height: 72px   (fixed bottom bar)                   │
│  Track Row Height:  40px   (virtualized list rows)              │
│  Min Content Width: 600px  (before horizontal scroll)           │
└─────────────────────────────────────────────────────────────────┘
```

---

## Component Library

### Buttons

```text
┌─────────────────────────────────────────────────────────────────┐
│  PRIMARY (main actions)                                         │
│  ┌──────────────────┐                                          │
│  │    Identify      │  bg: Primary, text: white, radius: 6px   │
│  └──────────────────┘  hover: Primary-Hover, press: Primary-Muted│
│                                                                 │
│  SECONDARY (supporting actions)                                 │
│  ┌──────────────────┐                                          │
│  │     Cancel       │  bg: Surface-2, text: Secondary          │
│  └──────────────────┘  border: 1px Default, hover: Surface-3   │
│                                                                 │
│  GHOST (minimal emphasis)                                       │
│  ┌──────────────────┐                                          │
│  │     Clear        │  bg: transparent, text: Muted            │
│  └──────────────────┘  hover: Surface-2 background appears     │
│                                                                 │
│  ICON BUTTON (compact)                                          │
│  ┌────┐                                                         │
│  │ ▶  │  32x32px, bg: transparent, hover: Surface-2            │
│  └────┘  active: Surface-3, icon color inherits context        │
│                                                                 │
│  DANGER (destructive)                                           │
│  ┌──────────────────┐                                          │
│  │     Delete       │  bg: Error, text: white                   │
│  └──────────────────┘  use sparingly, always confirm            │
│                                                                 │
│  CHIP (filter/tag style)                                        │
│  ┌────────┐                                                     │
│  │  FLAC  │  Small, rounded (radius: 12px), toggle state       │
│  └────────┘  active: Primary bg, inactive: Surface-2 bg        │
└─────────────────────────────────────────────────────────────────┘
```

### Cards

```text
┌─────────────────────────────────────────────────────────────────┐
│  STANDARD CARD                                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Background: Surface                                     │   │
│  │  Border: 1px Subtle                                      │   │
│  │  Radius: 8px                                             │   │
│  │  Padding: 16px                                           │   │
│  │  Shadow: 0 2px 8px rgba(0,0,0,0.3)                       │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ELEVATED CARD (modals, overlays)                               │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Background: Surface-2                                   │   │
│  │  Border: 1px Default                                     │   │
│  │  Radius: 12px                                            │   │
│  │  Padding: 24px                                           │   │
│  │  Shadow: 0 8px 32px rgba(0,0,0,0.5)                      │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  INLINE CARD (within content)                                   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Background: Surface (or transparent)                    │   │
│  │  Border: none (or 1px left accent)                       │   │
│  │  Radius: 4px                                             │   │
│  │  Padding: 12px                                           │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Input Fields

```text
┌─────────────────────────────────────────────────────────────────┐
│  TEXT INPUT                                                     │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Search tracks...                                    🔍  │   │
│  └─────────────────────────────────────────────────────────┘   │
│  bg: Surface, border: 1px Subtle, radius: 6px                  │
│  focus: border Primary, subtle glow                            │
│  placeholder: Muted color                                       │
│                                                                 │
│  SLIDER                                                         │
│  ○────────────●──────────○                                      │
│  track: Surface-3, fill: Primary, thumb: white                  │
│  height: 4px, thumb: 12px circle                                │
│                                                                 │
│  DROPDOWN / SELECT                                              │
│  ┌───────────────────────────────────────────────────┬───┐     │
│  │  Output Device                                    │ ▼ │     │
│  └───────────────────────────────────────────────────┴───┘     │
│  Same style as text input, dropdown: Surface-2 bg              │
└─────────────────────────────────────────────────────────────────┘
```

### Status Indicators

```text
┌─────────────────────────────────────────────────────────────────┐
│  DOTS                                                           │
│  ● Playing (Success, pulsing animation)                         │
│  ● Paused (Warning, static)                                     │
│  ● Error (Error, static)                                        │
│  ○ Inactive (Muted, hollow)                                     │
│                                                                 │
│  BADGES                                                         │
│  ┌────────┐                                                     │
│  │ FLAC   │  Lossless formats: Success bg, tiny radius          │
│  └────────┘                                                     │
│  ┌────────┐                                                     │
│  │  MP3   │  Lossy formats: Muted bg                            │
│  └────────┘                                                     │
│                                                                 │
│  PROGRESS                                                       │
│  ▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░ 50%                                       │
│  bg: Surface-3, fill: Primary, height: 4px                      │
│  For longer operations: add percentage text                     │
│                                                                 │
│  CONFIDENCE SCORE                                               │
│  ████████░░ 82%  (Warning color for 70-90%)                     │
│  ██████████ 97%  (Success color for 90%+)                       │
│  ████░░░░░░ 42%  (Error color for <70%)                         │
└─────────────────────────────────────────────────────────────────┘
```

---

## Layout Architecture

### Overall Structure

```text
┌─────────────────────────────────────────────────────────────────┐
│                         WINDOW                                  │
│  ┌────────┬──────────────────────────────────┬──────────────┐   │
│  │        │                                  │              │   │
│  │        │                                  │   CONTEXT    │   │
│  │  NAV   │          MAIN CONTENT            │    PANEL     │   │
│  │  BAR   │            AREA                  │  (optional)  │   │
│  │        │                                  │              │   │
│  │ 200px  │           flexible               │    320px     │   │
│  │        │                                  │              │   │
│  ├────────┴──────────────────────────────────┴──────────────┤   │
│  │                      PLAYER BAR                          │   │
│  │                        72px                              │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Navigation Bar (Sidebar)

```text
┌────────────────────┐
│                    │
│   MUSIC MINDER     │  Logo/Title, 20px
│                    │
│   ────────────     │
│                    │
│   📚 Library       │  Nav item, icon + label
│   🎵 Now Playing   │  Active: Primary bg, white text
│   ✨ Enrich        │  Inactive: transparent, Muted text
│   ⚙ Settings       │  Hover: Surface-2 bg
│                    │
│   ────────────     │  Divider
│                    │
│   ● Watching       │  Status indicator
│   3,428 tracks     │  Stats
│                    │
│   ────────────     │
│                    │
│   System: Good     │  Audio health
│   └ Excellent      │  Expandable details
│                    │
└────────────────────┘
Width: 200px (collapsible to 60px with icons only)
```

### Context Panel (Right Side)

Appears contextually based on selection or action.

```text
┌──────────────────────┐
│  QUICK ENRICH    ✕   │  Header with close button
│  ────────────────    │
│                      │
│  2 tracks selected   │  Selection summary
│                      │
│  ┌────────────────┐  │
│  │   COVER ART    │  │  Cover preview (if available)
│  │    PREVIEW     │  │
│  └────────────────┘  │
│                      │
│  CURRENT             │  Before state
│  ───────             │
│  Title: Track 01     │
│  Artist: Unknown     │
│  Album: —            │
│                      │
│  IDENTIFIED          │  After state (if identified)
│  ──────────          │
│  Title: Bohemian...  │  Changed fields highlighted
│  Artist: Queen       │
│  Album: A Night...   │
│  Year: 1975          │
│  Match: 97% ████████ │
│                      │
│  ┌────────────────┐  │
│  │   Identify     │  │  Primary action
│  └────────────────┘  │
│  ┌────────────────┐  │
│  │  Write Tags    │  │  Secondary (after identify)
│  └────────────────┘  │
│                      │
│  Send to Batch →     │  Ghost link
│                      │
└──────────────────────┘
Width: 320px, slides in from right
Animation: 200ms ease-out
```

---

## Pane Designs

### 1. Library Pane

The heart of the app. Browse, search, filter, play.

```text
┌──────────────────────────────────────────────────────────────────┐
│  LIBRARY                                         [+ Add Folder]  │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ 🔍 Search tracks, artists, albums...                       │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  [FLAC] [MP3] [Lossless] [♡ Favorites]              Clear All   │
│                                                                  │
│  3,428 tracks (showing 1,247)                    Sort: Artist ▼  │
│                                                                  │
│  ┌────┬──────────────────┬──────────┬────────────┬────┬────┬───┐│
│  │    │ Title          ▼ │ Artist   │ Album      │ Yr │ ⏱  │Fmt││
│  ├────┼──────────────────┼──────────┼────────────┼────┼────┼───┤│
│  │▶ + │ Bohemian Rhaps...│ Queen    │ A Night... │1975│5:55│FLAC│
│  │▶ + │ We Will Rock You │ Queen    │ News of... │1977│2:01│FLAC│
│  │▶ + │ Another One Bi...│ Queen    │ The Game   │1980│3:35│MP3 │
│  │▶ + │ Under Pressure   │ Queen... │ Hot Space  │1981│4:04│FLAC│
│  │    │                  │          │            │    │    │    │
│  │    │        (virtualized - smooth scrolling)        │    │    │
│  │    │                  │          │            │    │    │    │
│  └────┴──────────────────┴──────────┴────────────┴────┴────┴───┘│
│                                                                  │
│  ────────────────────────────────────────────────────────────── │
│                                                                  │
│  ORGANIZE                                              [Expand ▼]│
│  Pattern: {Artist}/{Album}/{Track} - {Title}                    │
│  ┌─────────────────────────────────────────────┐ [Browse] [Run] │
│  │ D:\Music\Organized                          │                │
│  └─────────────────────────────────────────────┘                │
└──────────────────────────────────────────────────────────────────┘
```

**Key Changes from Current:**

- Search bar is prominent, always visible
- Filter chips are pill-shaped, toggle on/off
- Track count shows filtered vs total
- Sort dropdown replaces clickable headers (cleaner)
- Organize section is collapsible (usually hidden)
- Row actions (play, queue) are icon-only, appear on hover
- Selection uses checkbox column for batch operations

### 2. Now Playing Pane

Focused listening experience. Big cover art, visualizations, queue.

```text
┌──────────────────────────────────────────────────────────────────┐
│                                                                  │
│         ┌─────────────────────┐                                  │
│         │                     │                                  │
│         │                     │                                  │
│         │     COVER ART       │     Bohemian Rhapsody            │
│         │      300x300        │     Queen                        │
│         │                     │     A Night at the Opera (1975)  │
│         │                     │                                  │
│         │                     │     FLAC • 44.1kHz • 16bit       │
│         └─────────────────────┘     Lossless                     │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                                                            │ │
│  │              ▊▊ ▊▊▊▊ ▊▊▊▊▊▊ ▊▊▊▊ ▊▊         │ │
│  │             ▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊ │ │
│  │            ▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊▊  │ │
│  │                    VISUALIZATION                           │ │
│  │                                                            │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  [Spectrum ●] [Waveform ○] [VU ○] [Off ○]                       │
│                                                                  │
│  ────────────────────────────────────────────────────────────── │
│                                                                  │
│  QUEUE                           Track 3 of 12    [🔀] [🔁] [✕] │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  1.  We Will Rock You              Queen           2:01    │ │
│  │  2.  We Are The Champions          Queen           2:59    │ │
│  │ ▶ 3. Bohemian Rhapsody            Queen           5:55 ◀  │ │
│  │  4.  Somebody To Love              Queen           4:56    │ │
│  │  5.  Don't Stop Me Now             Queen           3:29    │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

**Key Changes from Current:**

- Larger cover art (300px vs 200px)
- Track info to the right of cover, not below
- Visualization is a distinct section with mode toggles
- Queue is always visible, scrollable
- Current track highlighted with accent color and arrow
- Queue controls (shuffle, repeat, clear) in header

### 3. Enrich Pane (NEW)

Batch enrichment powerhouse.

```text
┌──────────────────────────────────────────────────────────────────┐
│  ENRICH LIBRARY                                                  │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ STATUS                                                    │   │
│  │ ● fpcalc ready    ● API key configured    ● Rate: OK     │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ TRACKS TO PROCESS                     [+ Add from Library]│   │
│  │ ────────────────────                                      │   │
│  │ ☑ Bohemian Rhapsody - Queen              [Remove]        │   │
│  │ ☑ Track 01 - Unknown Artist              [Remove]        │   │
│  │ ☑ untitled.flac - Unknown                [Remove]        │   │
│  │ ☐ Already identified track               [Remove]        │   │
│  │                                                           │   │
│  │ 4 tracks • 3 selected                    [Clear All]     │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  OPTIONS                                                        │
│  ○ Fill missing only (safe)   ● Overwrite all (replace tags)   │
│  ☑ Fetch cover art            ☐ Include low-confidence matches  │
│                                                                  │
│  [▶ Identify Selected]                                          │
│                                                                  │
│  ────────────────────────────────────────────────────────────── │
│                                                                  │
│  RESULTS                                   Progress: ▓▓▓░░ 2/4  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                                                           │   │
│  │  ✅ Bohemian Rhapsody                                     │   │
│  │     Queen → Queen (confirmed)         97% match          │   │
│  │     Album: — → A Night at the Opera                      │   │
│  │     Year: — → 1975                    [Review] [Write]   │   │
│  │                                                           │   │
│  │  ✅ Track 01                                              │   │
│  │     Unknown → Freddie Mercury         89% match          │   │
│  │     → "Barcelona"                     [Review] [Write]   │   │
│  │                                                           │   │
│  │  ⏳ untitled.flac                                         │   │
│  │     Identifying...                                        │   │
│  │                                                           │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  [💾 Write All Confirmed]        [📋 Export Report]             │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

### 4. Settings Pane

Clean, organized settings.

```text
┌──────────────────────────────────────────────────────────────────┐
│  SETTINGS                                                        │
│                                                                  │
│  AUDIO                                                          │
│  ─────                                                          │
│  Output Device        [Built-in Speakers           ▼]           │
│  Sample Rate          [44100 Hz                    ▼]           │
│  Buffer Size          [Medium (stable)             ▼]           │
│                                                                  │
│  LIBRARY                                                        │
│  ───────                                                        │
│  Watch Folders        D:\Music                     [+ Add]      │
│                       C:\Users\Me\Downloads        [Remove]     │
│  Auto-scan on start   [●]                                       │
│                                                                  │
│  ENRICHMENT                                                     │
│  ──────────                                                     │
│  AcoustID API Key     [●●●●●●●●●●●●●●●●●●●●]      [Show] [Test]│
│  MusicBrainz Rate     [1 request/sec (respectful)  ▼]           │
│  Cover Art Size       [500px (standard)            ▼]           │
│                                                                  │
│  APPEARANCE                                                     │
│  ──────────                                                     │
│  Theme                [Dark                        ▼]           │
│  Visualization        [Spectrum bars               ▼]           │
│  Accent Color         [●] Indigo  [○] Green  [○] Amber         │
│                                                                  │
│  ABOUT                                                          │
│  ─────                                                          │
│  Music Minder v0.1.4                                            │
│  "It really whips the llama's ass"                              │
│  [View Diagnostics] [Check for Updates]                         │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

---

## Player Bar (Always Visible)

The bottom bar should feel like a cohesive unit, always available.

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                                                                              │
│  ┌────┐  Bohemian Rhapsody                 2:45 ●────────────────○ 5:55      │
│  │  ♫ │  Queen • A Night at the Opera                                        │
│  └────┘                                                                      │
│         │◀◀│  │ ▶ │  │▶▶│        🔀  🔁        🔊 ▃▄▅▆█  [Speakers ▼] │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Elements (left to right):**

1. Mini cover art (48x48)
2. Track info (title, artist • album)
3. Transport controls (prev, play/pause, next)
4. Seek bar with current/total time
5. Queue controls (shuffle, repeat)
6. Volume slider with value
7. Device picker

**Interactions:**

- Click cover art → jump to Now Playing
- Click track info → jump to track in Library
- Hover volume → shows numeric value
- Right-click track → context menu

---

## Animations & Transitions

```text
┌─────────────────────────────────────────────────────────────────┐
│  TIMING                                                         │
│  ──────                                                         │
│  Instant:    0ms    (button press feedback)                     │
│  Fast:       100ms  (hover states, small UI changes)            │
│  Normal:     200ms  (panel slides, view transitions)            │
│  Slow:       300ms  (modal opens, major transitions)            │
│  Relaxed:    500ms  (loading states, progress)                  │
│                                                                 │
│  EASING                                                         │
│  ──────                                                         │
│  ease-out:   For things entering (panels sliding in)            │
│  ease-in:    For things leaving (panels sliding out)            │
│  ease:       For hover states, toggles                          │
│  linear:     For progress bars, continuous motion               │
│                                                                 │
│  SPECIFIC ANIMATIONS                                            │
│  ───────────────────                                            │
│  Context panel:     Slide from right, 200ms ease-out            │
│  View transition:   Fade, 150ms ease                            │
│  Track selection:   Background color, 100ms ease                │
│  Playing indicator: Gentle pulse, 2s infinite                   │
│  Visualization:     60fps, smooth interpolation                 │
│  Progress bar:      Linear, continuous                          │
│  Hover states:      100ms ease                                  │
│  Button press:      Scale(0.98), 50ms                           │
└─────────────────────────────────────────────────────────────────┘
```

---

## Interaction Patterns

### Selection

- **Single click**: Select track, show context panel
- **Double click**: Play track immediately
- **Ctrl+click**: Add to selection (multi-select)
- **Shift+click**: Range select
- **Checkbox column**: Explicit batch selection mode

### Context Menu (Right-click)

```text
┌────────────────────────┐
│ ▶ Play                 │
│ + Add to Queue         │
│ ▶▶ Play Next           │
│ ────────────────────── │
│ ✨ Identify            │
│ 📝 Edit Tags           │
│ 📁 Show in Explorer    │
│ ────────────────────── │
│ 🗑 Remove from Library │  <-- needs a confirm
└────────────────────────┘
```

### Keyboard Shortcuts

```text
Space       Play/Pause
←/→         Prev/Next track
↑/↓         Volume up/down
Shift+←/→   Seek ±5 seconds
/           Focus search
E           Open enrich panel
Escape      Close panel/clear selection
Enter       Play selected track
Delete      Remove from queue
Ctrl+A      Select all (in context)
```

---

## Responsive Behavior

### Compact Mode (width < 900px)

- Sidebar collapses to icons only (60px)
- Context panel becomes overlay/modal
- Player bar stacks (two rows)

### Expanded Mode (width > 1400px)

- More columns visible in track list
- Larger cover art in Now Playing
- Visualization gets more height

---

## Implementation Priority

### Phase 1: Foundation

1. Color system CSS variables / Rust constants
2. Typography scale
3. Spacing system
4. Button component variants
5. Input field styling

### Phase 2: Layout

1. New sidebar design
2. Player bar redesign
3. Context panel infrastructure

### Phase 3: Panes

1. Library pane refresh
2. Now Playing pane refresh
3. Settings pane cleanup
4. New Enrich pane

### Phase 4: Polish

1. Animations
2. Hover states
3. Keyboard shortcuts
4. Context menus

---

## Next Steps

1. **Confirm this direction** - Does this feel right?
2. **Create constants file** - Colors, spacing, typography in Rust
3. **Start with player bar** - High visibility, sets the tone
4. **Then sidebar** - Navigation foundation
5. **Then main panes** - One at a time

---

## Questions

1. **Sidebar**: Always visible, or collapsible toggle?
2. **Cover art size**: 300px in Now Playing, or larger?
3. **Visualization**: Keep current canvas, or redesign?
4. **Organize section**: Keep in Library, or move to separate pane?
5. **Font**: Use system fonts, or bundle Inter/custom font?
